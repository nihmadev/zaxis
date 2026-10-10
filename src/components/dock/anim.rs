use super::{style::Look, PanelId};
use crate::{context::Paint, Context, Id, Rect, SpringState, SpringValue, Vec2};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Pose {
    pub position: Vec2,
    pub size: Vec2,
}
crate::impl_spring_value!(Pose { position, size });
impl Pose {
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.position, self.size.max(Vec2::ZERO))
    }
    pub fn of(rect: Rect) -> Self {
        Self {
            position: rect.min,
            size: rect.size(),
        }
    }
}
pub(super) struct PanelMotion {
    pub rect: Rect,
    pub target: Rect,
    pub layer: Id,
    pub paint: Vec<(Id, Rect, Vec<Paint>)>,
    pub closing: bool,
    pub group: Id,
}
#[derive(Default)]
pub(crate) struct DockRuntime {
    pub last_frame: u64,
    pub(super) panels: HashMap<PanelId, PanelMotion>,
    pub(super) focus: HashMap<PanelId, Vec<Id>>,
    pub(super) tab_focus: HashMap<PanelId, Id>,
    pub(super) pending_focus: Option<PanelId>,
    pub(super) ring: Option<(PanelId, Rect)>,
    pub(super) preview: Option<Rect>,
    pub(super) seen: HashSet<PanelId>,
    pub(super) initialized: bool,
    pub(super) moving_float: Option<Id>,
    pub(super) float_bounds: HashMap<Id, Rect>,
}
impl DockRuntime {
    pub(crate) fn window_ids(&self) -> impl Iterator<Item = Id> + '_ {
        self.float_bounds.keys().copied()
    }
}
pub(super) fn rect(
    ctx: &mut Context,
    id: Id,
    target: Rect,
    initial: Option<SpringState<Pose>>,
    look: &Look,
) -> Rect {
    if look.off {
        ctx.remove_animation(id);
        return target;
    }
    match initial {
        Some(initial) => ctx
            .spring_transition_from(id, initial, Pose::of(target), look.spring)
            .value
            .value
            .rect(),
        None => ctx
            .spring_transition(id, Pose::of(target), look.spring)
            .value
            .value
            .rect(),
    }
}
pub(super) fn panel(
    ctx: &mut Context,
    id: Id,
    panel: PanelId,
    group: Id,
    target: Rect,
    runtime: &mut DockRuntime,
    look: &Look,
) -> Rect {
    let previous = runtime.panels.get(&panel);
    let initial = if let Some(previous) = previous {
        ctx.animation_status(id.with(("panel-motion", panel)))
            .is_none()
            .then_some(previous.rect)
    } else if let Some(previous) = runtime
        .panels
        .values()
        .find(|p| p.group == group && !p.closing)
    {
        Some(previous.rect)
    } else if runtime.initialized {
        // A short reveal near the final position; existing panels keep their trajectories.
        Some(target.shrink(target.size().min_element() * 0.015))
    } else {
        None
    };
    let initial = initial.map(|rect| {
        let source = previous.map_or(group, |p| p.group);
        let velocity = runtime
            .panels
            .iter()
            .filter(|(p, motion)| **p != panel && motion.group == source && !motion.closing)
            .find_map(|(p, _)| {
                ctx.sample_animation::<SpringState<Pose>>(id.with(("panel-motion", *p)))
            })
            .map_or_else(Pose::zero, |sample| sample.value.velocity);
        SpringState {
            value: Pose::of(rect),
            velocity,
        }
    });
    rect(ctx, id.with(("panel-motion", panel)), target, initial, look)
}

pub(super) fn velocity(ctx: &mut Context, id: Id, panel: PanelId) -> Pose {
    ctx.sample_animation::<SpringState<Pose>>(id.with(("panel-motion", panel)))
        .map_or_else(Pose::zero, |sample| sample.value.velocity)
}
/// Exit replay contains paint descriptions only; no viewer, hits, IME or AT nodes.
pub(super) fn exits(
    ui: &mut crate::Ui<'_>,
    id: Id,
    runtime: &mut DockRuntime,
    panels: &[PanelId],
    look: &Look,
) {
    let live = &runtime.seen;
    runtime.panels.retain(|panel, p| {
        if live.contains(panel) {
            return true;
        }
        if panels.contains(panel) {
            p.paint.clear();
            return true;
        }
        if look.off || p.paint.is_empty() {
            return false;
        }
        if !p.closing {
            p.target = p.rect.shrink(p.rect.size().min_element() * 0.015);
            p.closing = true;
        }
        let channel = id.with(("panel-motion", *panel));
        let animated = ui
            .context
            .spring_transition(channel, Pose::of(p.target), look.spring);
        let shown = animated.value.value.rect();
        let fade_id = id.with(("exit-opacity", *panel));
        let fade = ui
            .context
            .transition_from(
                fade_id,
                1.0_f32,
                0.0,
                crate::TweenOptions::new(std::time::Duration::from_millis(100)),
            )
            .value;
        let transform = crate::Transform::translation(shown.min - p.paint[0].1.min);
        for (index, (key, clip, paint)) in p.paint.iter().enumerate() {
            let (paint, transform, clip) = if index == 0 {
                let mut surface = Vec::new();
                look.surface.paint_shadow(shown, look.radius, &mut surface);
                look.surface.paint_body(
                    shown,
                    look.radius,
                    ui.style(),
                    ui.backdrop_blur,
                    &mut surface,
                );
                (surface, crate::Transform::IDENTITY, ui.clip_rect())
            } else {
                (
                    paint.clone(),
                    transform,
                    transform
                        .rect(*clip)
                        .intersect(shown)
                        .intersect(ui.clip_rect()),
                )
            };
            ui.context.paint(
                id.with(("exit", key)),
                p.layer,
                clip,
                vec![Paint::Visual {
                    paint,
                    transform,
                    opacity: fade,
                }],
            );
        }
        p.rect = shown;
        !animated.completed() || fade > 0.0
    });
    runtime.focus.retain(|p, _| runtime.seen.contains(p));
    runtime.tab_focus.retain(|p, _| runtime.seen.contains(p));
}
