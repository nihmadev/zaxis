use super::{
    anim, input::Request, layout::Group, style::Look, DockDropZone, DockOutput, DockRuntime,
};
use crate::{Id, Rect, TabDrag, TabDropZone, Ui};

pub(super) fn zones(
    ui: &mut Ui<'_>,
    id: Id,
    group: &Group,
    bounds: Rect,
    panels: &[crate::PanelId],
    requests: &mut Vec<Request>,
    preview: &mut Option<Rect>,
) {
    let key = id.with(("dock-zone", group.id));
    let own = Id::new(id);
    let out = crate::DropTarget::new(key, move |p: &TabDrag| p.group == own)
        .indicator(false)
        .attach(ui, ui.response(key, bounds, true));
    if out.acceptable {
        if let Some(pointer) = ui.context.input().pointer {
            *preview = DockDropZone::at(bounds, pointer).map(|z| z.preview(bounds));
        }
    }
    if let Some(d) = out.dropped {
        let p = panels.iter().copied().find(|p| Id::new(p) == d.payload.tab);
        // local geometry was published with the target; transform-free ratio is stable.
        let point = bounds.min + d.local;
        if let (Some(p), Some(zone)) = (p, DockDropZone::at(bounds, point)) {
            match zone {
                DockDropZone::Center => requests.push(Request::Move(p, group.active, None)),
                DockDropZone::Edge(side) => {
                    if let Some(target) = group.panels.iter().copied().find(|t| *t != p) {
                        requests.push(Request::Split(p, target, side));
                    }
                }
            }
        }
    }
    let quarter = bounds.size() * 0.25;
    let center = Rect::from_min_max(bounds.min + quarter, bounds.max - quarter);
    let out = TabDropZone::new(key.with("center"), group.id, id)
        .indicator(false)
        .at(ui, center);
    if out.hovering {
        *preview = Some(bounds);
    }
    if let Some(moved) = out.moved {
        // Resolve TabBar's value hash through the application's stable panel values.
        if let Some(p) = panels.iter().copied().find(|p| Id::new(p) == moved.tab) {
            requests.push(Request::Move(p, group.active, None));
        }
    }
}
pub(super) fn preview(
    ui: &mut Ui<'_>,
    id: Id,
    target: Option<Rect>,
    runtime: &mut DockRuntime,
    look: &Look,
    output: &mut DockOutput,
) {
    let channel = id.with("preview-rect");
    if let Some(target) = target {
        let initial = runtime.preview.map(|rect| crate::SpringState {
            value: anim::Pose::of(rect),
            velocity: anim::Pose {
                position: crate::Vec2::ZERO,
                size: crate::Vec2::ZERO,
            },
        });
        let rect = anim::rect(ui.context, channel, target, initial, look);
        runtime.preview = Some(rect);
        output.preview = Some(rect);
    }
    let tween = crate::TweenOptions::new(std::time::Duration::from_millis(90))
        .easing(crate::Easing::CubicOut);
    let amount = if look.off {
        if target.is_some() {
            1.0
        } else {
            0.0
        }
    } else {
        ui.context
            .transition(
                id.with("preview-opacity"),
                if target.is_some() { 1.0_f32 } else { 0.0 },
                tween,
            )
            .value
    };
    if let Some(rect) = runtime.preview.filter(|_| amount > 0.0) {
        super::paint::preview(ui, id.with("preview"), rect, amount, look);
    }
    if target.is_none() && amount == 0.0 {
        runtime.preview = None;
        ui.context.remove_animation(channel);
    }
}

pub(super) fn empty(
    ui: &mut Ui<'_>,
    id: Id,
    bounds: Rect,
    panels: &[crate::PanelId],
    requests: &mut Vec<Request>,
    preview: &mut Option<Rect>,
) {
    let key = id.with("empty-dock");
    let own = Id::new(id);
    let out = crate::DropTarget::new(key, move |p: &TabDrag| p.group == own)
        .indicator(false)
        .attach(ui, ui.response(key, bounds, true));
    if out.acceptable {
        *preview = Some(bounds);
    }
    if let Some(drop) = out.dropped {
        if let Some(panel) = panels
            .iter()
            .copied()
            .find(|p| Id::new(p) == drop.payload.tab)
        {
            requests.push(Request::Root(panel));
        }
    }
}
