use super::{anim::Pose, style::Look, DockOutput, DockRuntime, DockState};
use crate::{
    context::Paint, Border, Color, CornerRadius, Id, Rect, SpringState, SpringValue, Ui, Vec2,
};

/// Snap the stroke's outer edges to physical pixels, independently of its width.
pub(super) fn snap(rect: Rect, scale: f32) -> Rect {
    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    Rect::from_min_max(
        (rect.min * scale).round() / scale,
        (rect.max * scale).round() / scale,
    )
}
pub(super) fn alpha(mut color: Color, amount: f32) -> Color {
    color.0[3] = (f32::from(color.0[3]) * amount.clamp(0.0, 1.0)).round() as u8;
    color
}
#[derive(Clone, PartialEq)]
struct Channels {
    a: Vec2,
    b: Vec2,
}
crate::impl_spring_value!(Channels { a, b });
impl Channels {
    fn color(c: Color) -> Self {
        Self {
            a: Vec2::new(c.0[0] as f32, c.0[1] as f32),
            b: Vec2::new(c.0[2] as f32, c.0[3] as f32),
        }
    }
    fn to_color(&self) -> Color {
        Color([
            self.a.x.round().clamp(0.0, 255.0) as u8,
            self.a.y.round().clamp(0.0, 255.0) as u8,
            self.b.x.round().clamp(0.0, 255.0) as u8,
            self.b.y.round().clamp(0.0, 255.0) as u8,
        ])
    }
}
pub(super) fn paint(
    ui: &mut Ui<'_>,
    id: Id,
    state: &DockState,
    runtime: &mut DockRuntime,
    look: &Look,
    output: &mut DockOutput,
) {
    let Some(panel) = state.focused else {
        runtime.ring = None;
        return;
    };
    let Some(out) = output.panels.iter().find(|p| p.panel == panel) else {
        return;
    };
    let target = Rect::from_min_max(
        out.bounds.min - Vec2::splat(look.ring_gap),
        out.bounds.max + Vec2::splat(look.ring_gap),
    );
    let channel = id.with("focus-ring-rect");
    let mut offset = Pose::zero();
    if !look.off {
        if let Some((old_panel, old_rect)) = runtime.ring.filter(|(p, _)| *p != panel) {
            let velocity = ui
                .context
                .sample_animation::<SpringState<Pose>>(channel)
                .map_or_else(Pose::zero, |s| s.value.velocity)
                .add(&super::anim::velocity(ui.context, id, old_panel))
                .sub(&super::anim::velocity(ui.context, id, panel));
            ui.context.remove_animation(channel);
            offset = Pose::of(old_rect).sub(&Pose::of(target));
            ui.context.spring_transition_from(
                channel,
                SpringState {
                    value: offset.clone(),
                    velocity,
                },
                Pose::zero(),
                look.spring,
            );
        }
        offset = ui
            .context
            .spring_transition(channel, Pose::zero(), look.spring)
            .value
            .value;
    } else {
        ui.context.remove_animation(channel);
    }
    let rect = snap(
        Pose::of(target).add(&offset).rect(),
        ui.context.scale_factor(),
    );
    runtime.ring = Some((panel, rect));
    output.focus_ring = Some(rect);
    let focus = ui.context.focused();
    let focused = ui.context.input().focused
        && focus.is_some_and(|f| runtime.focus.values().any(|ids| ids.contains(&f)));
    let color = if focused {
        look.active
    } else {
        alpha(look.inactive, 0.45)
    };
    let color_id = id.with("focus-ring-color");
    let color = if look.off {
        ui.context.remove_animation(color_id);
        color
    } else {
        ui.context
            .spring_transition(
                color_id,
                Channels::color(color),
                if focused {
                    look.spring
                } else {
                    crate::SpringOptions::frequency(2.0, 1.0).thresholds(0.1, 0.1)
                },
            )
            .value
            .value
            .to_color()
    };
    let radius = CornerRadius {
        top_left: look.radius.top_left + look.ring_gap,
        top_right: look.radius.top_right + look.ring_gap,
        bottom_left: look.radius.bottom_left + look.ring_gap,
        bottom_right: look.radius.bottom_right + look.ring_gap,
    };
    let radius_id = id.with("focus-ring-radius");
    let radius = if look.off {
        ui.context.remove_animation(radius_id);
        radius
    } else {
        let c = ui
            .context
            .spring_transition(
                radius_id,
                Channels {
                    a: Vec2::new(radius.top_left, radius.top_right),
                    b: Vec2::new(radius.bottom_left, radius.bottom_right),
                },
                look.spring,
            )
            .value
            .value;
        CornerRadius {
            top_left: c.a.x,
            top_right: c.a.y,
            bottom_left: c.b.x,
            bottom_right: c.b.y,
        }
    };
    let scale = ui.context.scale_factor().max(0.01);
    let width = if look.ring_width == 0.0 {
        0.0
    } else {
        (look.ring_width * scale).round().max(1.0) / scale
    };
    let mut paints = Vec::new();
    // Concentric alpha strokes give an external glow without tinting the panel interior.
    for i in (1..=6).rev() {
        let spread = look.glow * i as f32 / 6.0;
        if spread <= 0.0 {
            continue;
        }
        let glow_rect = Rect::from_min_max(
            rect.min - Vec2::splat(spread),
            rect.max + Vec2::splat(spread),
        );
        let r = CornerRadius {
            top_left: radius.top_left + spread,
            top_right: radius.top_right + spread,
            bottom_left: radius.bottom_left + spread,
            bottom_right: radius.bottom_right + spread,
        };
        paints.push(Paint::Shape(
            crate::Shape::rect(glow_rect, Color::TRANSPARENT)
                .corner_radius(r)
                .border(Border::new(width, alpha(color, 0.018)))
                .into(),
        ));
    }
    let gradient_id = id.with("focus-ring-gradient");
    if look.gradient && !look.off && focused {
        let angle = ui
            .context
            .animate(gradient_id, || {
                crate::Rotation::new(std::time::Duration::from_secs(6))
            })
            .value;
        paints.push(Paint::Shape(crate::Shape::GradientBorder {
            rect,
            rounding: radius,
            width,
            gradient: crate::Gradient::new(color, look.inactive),
            angle,
        }));
    } else {
        ui.context.remove_animation(gradient_id);
        paints.push(Paint::Shape(
            crate::Shape::rect(rect, Color::TRANSPARENT)
                .corner_radius(radius)
                .border(Border::new(width, color))
                .into(),
        ));
    }
    let clip = if out.floating {
        ui.context.viewport()
    } else {
        ui.clip_rect()
    };
    output.focus_ring_color = Some(color);
    ui.context.paint(
        id.with("focus-ring"),
        runtime.panels.get(&panel).map_or(ui.window, |p| p.layer),
        clip,
        paints,
    );
}
