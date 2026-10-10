use super::{
    access,
    anim::{self, PanelMotion},
    drag, float, focus_ring,
    input::{self, Request},
    layout::{self, Group},
    style::Look,
    Dock, DockOutput, DockPanelOutput, DockRuntime, DockViewer, PanelId,
};
use crate::{Id, Rect, Sense, TabBar, TabItem, Transform, Ui, Vec2};

pub(super) fn run(dock: Dock<'_>, ui: &mut Ui<'_>, viewer: &mut dyn DockViewer) -> DockOutput {
    let id = ui.scope.with(("dock", dock.id));
    for issue in dock.state.validate(dock.state.panels()) {
        ui.context
            .report(crate::DiagnosticKind::InvalidValue, Some(id), None, || {
                format!("Dock: {issue:?}")
            });
    }
    dock.state.normalize();
    let look = Look::resolve(ui.style(), dock.style);
    let available = Vec2::new(ui.available_width(), ui.available_height());
    let bounds = ui.allocate_space(dock.size.unwrap_or(available).min(available));
    let mut output = DockOutput {
        bounds,
        ..Default::default()
    };
    let mut runtime = ui.context.containers.docks.remove(&id).unwrap_or_default();
    runtime.last_frame = ui.context.frame;
    runtime.seen.clear();
    let scope = access::root(ui, id, &dock.name);
    let mut groups = Vec::new();
    ui.context.begin_placement(ui.window);
    if let Some(node) = &mut dock.state.root {
        layout::child(
            ui,
            id.with("layout"),
            bounds.shrink(look.gap * 0.5),
            ui.clip_rect(),
            |ui| layout::tree(ui, node, &look, &mut groups, &mut output),
        );
    }
    let split_placement = ui.context.end_placement();
    let mut requests = Vec::new();
    let mut navigation: Vec<Group> = groups
        .iter()
        .map(|g| Group {
            id: g.id,
            panels: g.panels.clone(),
            active: g.active,
            bounds: g.bounds,
        })
        .collect();
    for float in &dock.state.floats {
        layout::navigation(&float.node, float.bounds, look.gap, &mut navigation);
    }
    input::keyboard(
        ui,
        dock.state,
        &navigation,
        dock.actions,
        &runtime,
        &mut requests,
    );
    let panels = dock.state.panels();
    let mut preview = None;
    for group in &groups {
        group_show(
            ui,
            id,
            group,
            &panels,
            &mut runtime,
            &look,
            viewer,
            &mut requests,
            &mut output,
            false,
        );
    }
    ui.context
        .place(split_placement, Vec2::ZERO, ui.clip_rect());
    if groups.is_empty() {
        drag::empty(ui, id, bounds, &panels, &mut requests, &mut preview);
    }
    for group in &groups {
        let shown = output
            .panels
            .iter()
            .find(|p| p.group == group.id)
            .map_or(group.bounds, |p| p.bounds);
        let body = Rect::from_min_max(
            shown.min + Vec2::new(0.0, layout::tab_height(ui, &look).min(shown.size().y)),
            shown.max,
        );
        drag::zones(ui, id, group, body, &panels, &mut requests, &mut preview);
    }
    ui.a11y_end(scope, Some(bounds));
    float::show(
        ui,
        id,
        dock.state,
        &groups,
        &panels,
        &mut runtime,
        &look,
        viewer,
        &mut requests,
        &mut preview,
        &mut output,
    );
    anim::exits(ui, id, &mut runtime, &panels, &look);
    runtime.float_bounds.retain(|wid, _| {
        let keep = dock
            .state
            .floats
            .iter()
            .any(|f| id.with(("float-window", f.node.id())) == *wid)
            || runtime
                .panels
                .values()
                .any(|p| p.layer == *wid && p.closing);
        if !keep {
            ui.context.forget_window(*wid);
        }
        keep
    });
    input::apply(ui, dock.state, requests, viewer, &mut output, &mut runtime);
    focus_ring::paint(ui, id, dock.state, &mut runtime, &look, &mut output);
    drag::preview(ui, id, preview, &mut runtime, &look, &mut output);
    runtime.initialized = true;
    ui.context.containers.docks.insert(id, runtime);
    output
}

#[allow(clippy::too_many_arguments)]
pub(super) fn group_show(
    ui: &mut Ui<'_>,
    id: Id,
    group: &Group,
    panels: &[PanelId],
    runtime: &mut DockRuntime,
    look: &Look,
    viewer: &mut dyn DockViewer,
    requests: &mut Vec<Request>,
    output: &mut DockOutput,
    floating: bool,
) {
    let p = group.active;
    runtime.seen.insert(p);
    let target = group.bounds;
    let shown = anim::panel(ui.context, id, p, group.id, target, runtime, look);
    let outer_clip = if floating {
        ui.context.viewport()
    } else {
        ui.clip_rect()
    };
    let clip = shown.intersect(outer_clip);
    super::paint::surface(ui, id.with(("surface", p)), shown, look, outer_clip);
    let height = layout::tab_height(ui, look);
    let mut tab_rect = Rect::from_min_size(
        shown.min,
        Vec2::new(shown.size().x, height.min(shown.size().y)),
    );
    if floating {
        let grip = Rect::from_min_max(
            Vec2::new((tab_rect.max.x - 24.0).max(tab_rect.min.x), tab_rect.min.y),
            tab_rect.max,
        );
        super::float::drag_header(ui, grip);
        tab_rect.max.x = grip.min.x;
    }
    let mut selected = p;
    let tab_output = layout::child(ui, id.with(("tabs", group.id)), tab_rect, clip, |ui| {
        let tabs: Vec<_> = group
            .panels
            .iter()
            .map(|panel| {
                let mut item =
                    TabItem::new(*panel, viewer.title(panel)).closable(viewer.closable(panel));
                if let Some(icon) = viewer.icon(panel) {
                    item = item.icon(icon);
                }
                item
            })
            .collect();
        let out = TabBar::new(group.id, &mut selected, tabs)
            .group(id)
            .reorderable(true)
            .keyboard_close(false)
            .overflow_menu(true)
            .style(look.tabs)
            .show(ui);
        for (panel, response) in group.panels.iter().zip(&out.responses) {
            runtime.tab_focus.insert(*panel, response.id);
            input::menu(
                ui,
                *panel,
                *response,
                group.panels.iter().copied().find(|other| other != panel),
                viewer,
                requests,
            );
        }
        out
    });
    if floating {
        if let Some(tail) = tab_output.free_space {
            super::float::drag_header(ui, tail);
        }
    }
    if runtime.pending_focus == Some(p) {
        if let Some(tab) = runtime.tab_focus.get(&p) {
            ui.context.request_focus(*tab);
        }
        runtime.pending_focus = None;
    }
    if tab_output.selected.is_some() {
        requests.push(Request::Activate(selected));
    }
    if let Some(key) = tab_output.closed {
        if let Some(p) = panels.iter().copied().find(|p| Id::new(p) == key) {
            requests.push(Request::Close(p));
        }
    }
    if let Some(moved) = tab_output.moved {
        let moving = ui.context.input().modifiers.control_key()
            && ui.context.input().modifiers.shift_key()
            && ui.context.dragging().is_none();
        if !moving || requests.is_empty() {
            if let Some(p) = panels.iter().copied().find(|p| Id::new(p) == moved.tab) {
                let before = moved
                    .before
                    .and_then(|b| panels.iter().copied().find(|p| Id::new(p) == b));
                requests.push(Request::Move(p, group.active, before));
            }
        }
    }
    if let Some(release) = tab_output.released_outside {
        if let Some(p) = panels.iter().copied().find(|p| Id::new(p) == release.tab) {
            requests.push(Request::Float(
                p,
                Rect::from_min_size(
                    release.position - Vec2::new(40.0, 16.0),
                    Vec2::new(380.0, 260.0),
                ),
            ));
        }
    }
    let body = layout::body(target, height, look.padding);
    let displayed_body = layout::body(shown, height, look.padding);
    let delta = shown.min - target.min;
    let panel_scope = id.with(("content", p));
    ui.context.begin_placement(ui.window);
    ui.context.visuals.depth += 1;
    ui.context.visuals.clips.push((ui.window, body));
    let response = layout::child(ui, panel_scope, body, body, |ui| {
        let response = ui.interact(body, "panel", Sense::CLICK);
        let scope = access::panel(ui, panel_scope, body, &viewer.title(&p), true);
        viewer.ui(ui, &p);
        ui.a11y_end(scope, Some(body));
        response
    });
    ui.context.visuals.clips.pop();
    ui.context.visuals.depth -= 1;
    let placement = ui.context.end_placement();
    let mut hits = ui.context.placement_hit_ids(&placement);
    hits.extend(tab_output.responses.iter().map(|r| r.id));
    let focused = ui.context.focused().is_some_and(|f| hits.contains(&f));
    if response.clicked() {
        requests.push(Request::Activate(p));
        if let Some(tab) = runtime.tab_focus.get(&p) {
            ui.context.request_focus(*tab);
        }
    } else if focused
        && !requests.iter().any(|r| {
            matches!(
                r,
                Request::Activate(_) | Request::Close(_) | Request::Move(..) | Request::Split(..)
            )
        })
    {
        requests.push(Request::Activate(p));
    }
    runtime.focus.insert(p, hits);
    let mut paint: Vec<_> = placement
        .paints
        .iter()
        .filter(|item| item.layer == ui.window && item.material.is_none())
        .map(|item| (item.id, item.clip, item.paint.clone()))
        .collect();
    let mut surface = Vec::new();
    look.surface.paint_shadow(target, look.radius, &mut surface);
    look.surface.paint_body(
        target,
        look.radius,
        ui.style(),
        ui.backdrop_blur,
        &mut surface,
    );
    paint.insert(0, (id.with(("surface", p)), target, surface));
    runtime.panels.insert(
        p,
        PanelMotion {
            rect: shown,
            target,
            layer: ui.window,
            paint,
            closing: false,
            group: group.id,
        },
    );
    // Hidden tabs share the current group pose without building their contents.
    for hidden in group.panels.iter().copied().filter(|hidden| *hidden != p) {
        runtime.panels.insert(
            hidden,
            PanelMotion {
                rect: shown,
                target,
                layer: ui.window,
                paint: Vec::new(),
                closing: false,
                group: group.id,
            },
        );
    }
    ui.context.place_visual(
        placement,
        Transform::translation(delta),
        1.0,
        displayed_body.intersect(outer_clip),
        true,
    );
    output.panels.push(DockPanelOutput {
        panel: p,
        group: group.id,
        bounds: shown,
        target_bounds: target,
        content_bounds: displayed_body,
        floating,
    });
}
