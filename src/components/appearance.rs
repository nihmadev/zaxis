use super::theme::{ControlStyle, SurfaceStyle};
use super::{HoverFill, HoverStyle, Response, Ui};
use crate::{
    context::Paint, Border, Color, CornerRadius, Gradient, Interpolate, Rect, Shadow, Shape,
};

/// Resolved hover properties share one track, including gradient/shadow fades.
/// Hit geometry and control values never move with hover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Appearance {
    pub fill: Gradient,
    pub border: Border,
    pub text_color: Color,
    pub shadow: Shadow,
    pub rounding: CornerRadius,
    pub blur: f32,
    pub opacity: f32,
}
impl Interpolate for Appearance {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        Self {
            fill: self.fill.interpolate(&to.fill, t),
            border: self.border.interpolate(&to.border, t),
            text_color: self.text_color.interpolate(&to.text_color, t),
            shadow: self.shadow.interpolate(&to.shadow, t),
            rounding: self.rounding.interpolate(&to.rounding, t),
            blur: self.blur.interpolate(&to.blur, t),
            opacity: self.opacity.interpolate(&to.opacity, t),
        }
    }
}
impl Appearance {
    pub fn new(fill: Color, border: Border, text_color: Color) -> Self {
        Self {
            fill: Gradient::new(fill, fill),
            border,
            text_color,
            rounding: CornerRadius::ZERO,
            blur: 0.0,
            opacity: 1.0,
            shadow: Shadow {
                color: Color::TRANSPARENT,
                ..Shadow::default()
            },
        }
    }
    pub fn apply(&mut self, patch: SurfaceStyle) {
        if let Some(v) = patch.fill {
            self.fill = v;
        }
        if let Some(v) = patch.foreground {
            self.text_color = v;
        }
        if let Some(v) = patch.border {
            self.border = v;
        }
        if let Some(v) = patch.shadow {
            self.shadow = v;
        }
        if let Some(v) = patch.rounding {
            self.rounding = v;
        }
        if let Some(v) = patch.blur {
            self.blur = super::blur::normalize_radius(v);
        }
        if let Some(v) = patch.opacity {
            self.opacity = v.clamp(0.0, 1.0);
        }
    }
    pub fn surface(self) -> SurfaceStyle {
        SurfaceStyle {
            fill: Some(self.fill),
            foreground: Some(self.text_color),
            border: Some(self.border),
            shadow: Some(self.shadow),
            rounding: Some(self.rounding),
            blur: Some(self.blur),
            opacity: Some(self.opacity),
        }
    }
    pub fn paint_shadow(self, rect: Rect, rounding: CornerRadius, paint: &mut Vec<Paint>) {
        if self.shadow.color.0[3] > 0 {
            paint.push(Paint::Shape(Shape::Shadow {
                rect,
                rounding,
                shadow: Shadow {
                    color: alpha(self.shadow.color, self.opacity),
                    ..self.shadow
                },
            }));
        }
    }
    pub fn paint_body(
        self,
        rect: Rect,
        rounding: CornerRadius,
        style: &super::Style,
        blur: f32,
        paint: &mut Vec<Paint>,
    ) {
        if self.fill.start == self.fill.end {
            paint.push(Paint::Shape(Shape::Rect {
                rect,
                rounding,
                fill: alpha(style.backdrop_fill(self.fill.start, blur), self.opacity),
                border: Border {
                    color: alpha(self.border.color, self.opacity),
                    ..self.border
                },
            }));
        } else {
            paint.push(Paint::Shape(Shape::Gradient {
                rect,
                rounding,
                gradient: Gradient {
                    start: alpha(style.backdrop_fill(self.fill.start, blur), self.opacity),
                    end: alpha(style.backdrop_fill(self.fill.end, blur), self.opacity),
                    ..self.fill
                },
            }));
            paint.push(Paint::Shape(Shape::Rect {
                rect,
                rounding,
                fill: Color::TRANSPARENT,
                border: Border {
                    color: alpha(self.border.color, self.opacity),
                    ..self.border
                },
            }));
        }
    }
}

pub(super) fn alpha(mut color: Color, opacity: f32) -> Color {
    color.0[3] = (f32::from(color.0[3]) * opacity.clamp(0.0, 1.0)).round() as u8;
    color
}

impl Ui<'_> {
    /// One appearance track for all independent state layers.
    pub(super) fn animate_control(
        &mut self,
        response: Response,
        preset: HoverStyle,
        explicit_hover: bool,
        control: ControlStyle,
        state: super::theme::ControlState,
        mut base: Appearance,
        hovered: Color,
    ) -> Appearance {
        let focus = state.focus;
        let mut animated_state = state;
        animated_state.focus = false;
        base = resolve_control(
            self.style(),
            base,
            preset,
            explicit_hover,
            control,
            animated_state,
            hovered,
        );
        if focus {
            let mut patch = control.focus;
            patch.border = None;
            base.apply(patch);
        }
        let motion = if self.context.palette_transition.is_some() {
            crate::TweenOptions::new(std::time::Duration::ZERO)
        } else {
            self.style().motion.hover.clone()
        };
        let mut appearance = self.transition_property(
            response.id.with((
                "appearance",
                self.context.style_revision,
                self.local_style_revision,
            )),
            response.rect,
            base,
            motion,
        );
        // Focus ring is an immediate independent layer, never a faint tween.
        if focus {
            appearance.border = control.focus.border.unwrap_or(self.style().focus_border);
        }
        appearance
    }
    pub(super) fn animate_hover(
        &mut self,
        response: Response,
        preset: HoverStyle,
        mut normal: Appearance,
        hovered: Color,
    ) -> Appearance {
        if let Some(HoverFill::Gradient(g)) = preset.fill {
            normal.fill.direction = g.direction;
        }
        if let Some(shadow) = preset.shadow {
            normal.shadow = Shadow {
                color: Color::TRANSPARENT,
                ..shadow
            };
        }
        let state = super::theme::ControlState::from_response(response, false);
        self.animate_control(
            response,
            preset,
            true,
            ControlStyle::default(),
            state,
            normal,
            hovered,
        )
    }
}

fn apply_hover(base: &mut Appearance, preset: HoverStyle, hovered: Color) {
    match preset.fill {
        Some(HoverFill::Theme) => base.fill = Gradient::new(hovered, hovered),
        Some(HoverFill::Solid(c)) => base.fill = Gradient::new(c, c),
        Some(HoverFill::Gradient(g)) => base.fill = g,
        None => {}
    }
    if let Some(v) = preset.border {
        base.border = v;
    }
    if let Some(v) = preset.text_color {
        base.text_color = v;
    }
    if let Some(v) = preset.shadow {
        base.shadow = v;
    }
}

pub(crate) fn resolve_control(
    style: &super::Style,
    mut base: Appearance,
    preset: HoverStyle,
    explicit_hover: bool,
    control: ControlStyle,
    state: super::theme::ControlState,
    hovered: Color,
) -> Appearance {
    base.apply(control.idle);
    if state.selected {
        base.apply(control.selected);
    }
    base.apply(match state.status {
        super::theme::SemanticStatus::Normal => SurfaceStyle::default(),
        super::theme::SemanticStatus::Success => control.success,
        super::theme::SemanticStatus::Warning => control.warning,
        super::theme::SemanticStatus::Error => control.error,
    });
    if !state.enabled {
        base.text_color = style.disabled_text;
        base.apply(control.disabled);
    } else if state.pressed {
        base.apply(control.pressed);
    } else if state.hovered {
        if !explicit_hover {
            apply_hover(&mut base, preset, hovered);
        }
        base.apply(control.hover);
        if explicit_hover {
            apply_hover(&mut base, preset, hovered);
        }
    }
    if state.focus {
        base.border = style.focus_border;
        base.apply(control.focus);
    }
    base
}
