use super::{style::Look, DockEvent, DockNode, DockOutput, PanelId};
use crate::{
    Id, Layout, Rect, SplitHandle, SplitPane, SplitPanel, SplitSize, SplitSurface, Ui, Vec2,
};

pub(super) struct Group {
    pub id: Id,
    pub panels: Vec<PanelId>,
    pub active: PanelId,
    pub bounds: Rect,
}
/// SplitPane supplies allocation, min constraints, capture, AT boundaries and reset.
/// Content is built later so an animated panel can travel outside its new split cell.
pub(super) fn tree(
    ui: &mut Ui<'_>,
    node: &mut DockNode,
    look: &Look,
    groups: &mut Vec<Group>,
    output: &mut DockOutput,
) {
    match node {
        DockNode::Tabs { id, panels, active } => {
            if let Some(active) = active.or_else(|| panels.first().copied()) {
                groups.push(Group {
                    id: *id,
                    panels: panels.clone(),
                    active,
                    bounds: Rect::from_min_size(
                        ui.layout.cursor,
                        Vec2::new(ui.available_width(), ui.available_height()),
                    ),
                });
            }
        }
        DockNode::Split { id, axis, children } => {
            let specs: Vec<_> = children
                .iter()
                .map(|c| {
                    let min = minimum(&c.node, look);
                    SplitPanel::new(c.node.id())
                        .default_size(SplitSize::Weight(1.0))
                        .size(SplitSize::Weight(c.fraction))
                        .min_size(axis.main(min))
                })
                .collect();
            let mut style = ui.style().split;
            style.container = SplitSurface::default();
            style.panel = SplitSurface::default();
            style.gap = look.gap;
            style.handle.kind = if look.preset == super::DockPreset::Tiled {
                SplitHandle::Invisible
            } else {
                SplitHandle::Line
            };
            style.handle.idle = ui.style().border.color;
            style.handle.hover = ui.style().accent;
            style.handle.pressed = ui.style().accent;
            style.handle.focus = ui.style().focus_border.color;
            style.handle.hit_width = look.gap.max(6.0);
            let out = SplitPane::new(*id, *axis)
                .panels(specs)
                .style(style)
                .double_click_reset(true)
                .show(ui, |split| {
                    for child in children.iter_mut() {
                        split.panel(child.node.id(), |ui| {
                            tree(ui, &mut child.node, look, groups, output)
                        });
                    }
                });
            if out.boundaries.iter().any(|b| b.changed) {
                let total: f32 = out.panels.iter().map(|p| p.size).sum();
                if total > 0.0 {
                    for (c, p) in children.iter_mut().zip(&out.panels) {
                        c.fraction = (p.size / total).max(f32::MIN_POSITIVE);
                    }
                }
                if !output.events.contains(&DockEvent::LayoutChanged) {
                    output.events.push(DockEvent::LayoutChanged);
                }
            }
        }
    }
}
pub(super) fn minimum(node: &DockNode, look: &Look) -> Vec2 {
    match node {
        DockNode::Tabs { .. } => look.minimum,
        DockNode::Split { axis, children, .. } => {
            let mut main = look.gap * children.len().saturating_sub(1) as f32;
            let mut cross = 0.0_f32;
            for c in children {
                let min = minimum(&c.node, look);
                main += axis.main(min);
                cross = cross.max(axis.cross(min));
            }
            axis.size(main, cross)
        }
    }
}
pub(super) fn child<R>(
    ui: &mut Ui<'_>,
    scope: Id,
    bounds: Rect,
    clip: Rect,
    build: impl FnOnce(&mut Ui<'_>) -> R,
) -> R {
    let spacing = ui.style().spacing;
    let mut child = Ui {
        context: ui.context,
        window: ui.window,
        scope,
        sequence: 0,
        layout: crate::layout::LayoutCursor::new(bounds, Layout::Vertical, spacing),
        clip,
        enabled: ui.enabled,
        backdrop_blur: ui.backdrop_blur,
        hover_style: ui.hover_style,
        local_style: ui.local_style.clone(),
        local_style_revision: ui.local_style_revision,
        flow: None,
    };
    child.begin_layout(crate::Align::Start);
    let out = build(&mut child);
    child.finish_layout();
    out
}
pub(super) fn tab_height(ui: &Ui<'_>, look: &Look) -> f32 {
    look.tabs.height.unwrap_or(ui.style().control_height + 2.0)
}
pub(super) fn body(bounds: Rect, height: f32, padding: f32) -> Rect {
    Rect::from_min_max(
        bounds.min + Vec2::new(0.0, height.min(bounds.size().y)),
        bounds.max,
    )
    .shrink(padding)
}

/// Geometry for directional focus, including retained floating groups.
pub(super) fn navigation(node: &DockNode, bounds: Rect, gap: f32, out: &mut Vec<Group>) {
    match node {
        DockNode::Tabs { id, panels, active } => {
            if let Some(active) = active.or_else(|| panels.first().copied()) {
                out.push(Group {
                    id: *id,
                    panels: panels.clone(),
                    active,
                    bounds,
                });
            }
        }
        DockNode::Split { axis, children, .. } => {
            let total = children
                .iter()
                .map(|c| c.fraction)
                .sum::<f32>()
                .max(f32::MIN_POSITIVE);
            let extent =
                (axis.main(bounds.size()) - gap * children.len().saturating_sub(1) as f32).max(0.0);
            let mut offset = 0.0;
            for child in children {
                let size = axis.size(extent * child.fraction / total, axis.cross(bounds.size()));
                navigation(
                    &child.node,
                    Rect::from_min_size(bounds.min + axis.size(offset, 0.0), size),
                    gap,
                    out,
                );
                offset += axis.main(size) + gap;
            }
        }
    }
}
