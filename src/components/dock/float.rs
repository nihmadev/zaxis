use super::{
    input::Request,
    layout::{self, Group},
    options,
    style::Look,
    DockDropZone, DockOutput, DockRuntime, DockState, DockViewer, PanelId,
};
use crate::{Border, Color, Id, Padding, SurfaceStyle, Ui, Window, WindowStyle};

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    ui: &mut Ui<'_>,
    id: Id,
    state: &mut DockState,
    docked: &[Group],
    panels: &[PanelId],
    runtime: &mut DockRuntime,
    look: &Look,
    viewer: &mut dyn DockViewer,
    requests: &mut Vec<Request>,
    preview: &mut Option<crate::Rect>,
    output: &mut DockOutput,
) {
    let mut raised = None;
    for (index, float) in state.floats.iter_mut().enumerate() {
        let wid = id.with(("float-window", float.node.id()));
        if runtime
            .float_bounds
            .get(&wid)
            .is_some_and(|previous| *previous != float.bounds)
        {
            if let Some(window) = ui.context.windows.get_mut(&wid) {
                window.rect = float.bounds;
            }
        }
        let inherited = ui.style().clone();
        let mut groups = Vec::new();
        Window::new(
            options::active(&float.node).map_or_else(|| "Panel".into(), |p| viewer.title(&p)),
        )
        .id(wid)
        .default_position(float.bounds.min)
        .default_size(float.bounds.size())
        .min_size(layout::minimum(&float.node, look) + crate::Vec2::splat(12.0))
        .title_bar(false)
        .padding(Padding::all(0.0))
        .corner_radius(look.radius)
        .style(WindowStyle {
            body: SurfaceStyle {
                border: Some(Border::NONE),
                shadow: Some(crate::Shadow {
                    color: Color::TRANSPARENT,
                    ..Default::default()
                }),
                ..SurfaceStyle::fill(Color::TRANSPARENT)
            },
            ..Default::default()
        })
        .effective_style(inherited)
        .show(ui.context, |ui| {
            layout::tree(ui, &mut float.node, look, &mut groups, output);
            for group in &groups {
                super::show::group_show(
                    ui, id, group, panels, runtime, look, viewer, requests, output, true,
                );
            }
            let displayed = groups
                .iter()
                .filter_map(|group| runtime.panels.get(&group.active).map(|p| p.rect))
                .reduce(|a, b| crate::Rect::from_min_max(a.min.min(b.min), a.max.max(b.max)));
            if let Some(rect) = displayed {
                ui.context.set_window_displayed_bounds(
                    wid,
                    crate::Rect::from_min_max(rect.min, rect.max + crate::Vec2::splat(12.0)),
                );
            }
        });
        if let Some(window) = ui.context.windows.get(&wid) {
            if window.rect != float.bounds
                && !output.events.contains(&super::DockEvent::LayoutChanged)
            {
                output.events.push(super::DockEvent::LayoutChanged);
            }
            float.bounds = window.rect;
        }
        runtime.float_bounds.insert(wid, float.bounds);
        let pointer_click = ui.context.input().primary_pressed
            && ui
                .context
                .input()
                .pointer
                .is_some_and(|p| ui.context.top_window(p) == Some(wid));
        if pointer_click {
            raised = Some(index);
        }
        let moving = ui.context.window_moving(wid);
        let was_moving = runtime.moving_float == Some(wid);
        if moving {
            runtime.moving_float = Some(wid);
        }
        if moving || (was_moving && ui.context.input().primary_released) {
            if let Some(point) = ui.context.input().pointer {
                if docked.is_empty() && output.bounds.contains(point) {
                    *preview = Some(output.bounds);
                    if !moving {
                        if let Some(panel) = options::active(&float.node) {
                            requests.push(Request::Root(panel));
                        }
                    }
                }
                if let Some(target) = docked.iter().find(|g| g.bounds.contains(point)) {
                    if let (Some(panel), Some(zone)) = (
                        options::active(&float.node),
                        DockDropZone::at(target.bounds, point),
                    ) {
                        *preview = Some(zone.preview(target.bounds));
                        if !moving {
                            requests.push(match zone {
                                DockDropZone::Center => Request::Move(panel, target.active, None),
                                DockDropZone::Edge(side) => {
                                    Request::Split(panel, target.active, side)
                                }
                            });
                        }
                    }
                }
            }
            if !moving {
                runtime.moving_float = None;
            }
        }
    }
    if let Some(index) = raised {
        let float = state.floats.remove(index);
        state.floats.push(float);
    }
    let order: Vec<_> = state
        .floats
        .iter()
        .map(|f| id.with(("float-window", f.node.id())))
        .collect();
    ui.context.order_windows(&order);
}

pub(super) fn drag_header(ui: &mut Ui<'_>, rect: crate::Rect) {
    let clip = ui.clip;
    ui.clip = ui.context.viewport();
    ui.drag_window(rect);
    ui.clip = clip;
}
