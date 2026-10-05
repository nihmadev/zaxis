use super::theme::{ControlStyle, SurfaceStyle};
use super::{HoverFill, HoverStyle, Response, Ui};
use crate::{
    context::Paint, Border, Color, CornerRadius, Gradient, Interpolate, Rect, Shadow, Shape,
};

/// Resolved hover properties share one track, including gradient/shadow fades.
/// Hit geometry and control values never move with hover.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    pub fill: Gradient,
    pub border: Border,
    pub text_color: Color,
    pub shadow: Shadow,
    pub rounding: CornerRadius,
    pub blur: f32,
    pub opacity: f32,
    /// Keyboard-focus ring drawn outside the body; never interpolated.
    pub ring: Option<Border>,
    /// Validation color of a field-like control: tints the border and adds a soft
    /// ring. Set by the control before `animate_control`; never interpolated.
    pub status: Option<Color>,
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
            ring: to.ring,
            status: to.status,
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
            ring: None,
            status: None,
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
    /// Soft status ring, drawn inside the body so a container edge or scroll
    /// viewport flush with the control can never clip it. It slides under the
    /// border by half its width, so no gap shows between the two.
    fn paint_ring(self, rect: Rect, rounding: CornerRadius, paint: &mut Vec<Paint>) {
        let Some(ring) = self.ring.filter(|r| r.width > 0.0) else {
            return;
        };
        let under = self.border.width.max(0.0) * 0.5;
        let inset = rect.shrink(under);
        let radius = |r: f32| (r - under).max(0.0);
        paint.push(Paint::Shape(Shape::Rect {
            rect: inset,
            rounding: CornerRadius {
                top_left: radius(rounding.top_left),
                top_right: radius(rounding.top_right),
                bottom_right: radius(rounding.bottom_right),
                bottom_left: radius(rounding.bottom_left),
            },
            fill: Color::TRANSPARENT,
            border: Border {
                color: alpha(ring.color, self.opacity),
                width: ring.width + under,
            },
        }));
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
        self.paint_ring(rect, rounding, paint);
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
        let status = base.status;
        base = resolve_control(
            self.style(),
            base,
            preset,
            explicit_hover,
            control,
            animated_state,
            hovered,
        );
        base.status = status;
        if let Some(color) = status {
            // The status border survives hover, press and disabled; it fades like any other color.
            base.border = self.status_border(color, base.border);
        }
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
        // Rings are immediate independent layers, never a faint tween.
        if let Some(color) = status {
            // Invalid: status border and a soft ring in the same color, stronger while
            // focused. A disabled control keeps the border but draws no ring.
            appearance.border = self.status_border(color, appearance.border);
            if focus {
                appearance.border.width =
                    appearance.border.width.max(self.style().focus_border.width);
            }
            if state.enabled {
                appearance.ring = Some(self.status_ring(color, focus));
            }
        } else if focus {
            // Focus is the solid border alone; a translucent outer ring would double it.
            appearance.border = control.focus.border.unwrap_or(self.style().focus_border);
        }
        appearance
    }

    /// Status of a field-like control: its own, else the enclosing `Field`'s.
    pub(super) fn field_status(
        &self,
        own: super::theme::SemanticStatus,
    ) -> super::theme::SemanticStatus {
        if own == super::theme::SemanticStatus::Normal {
            self.context.field_status
        } else {
            own
        }
    }

    /// Theme color of a status; `None` for `Normal`.
    pub(super) fn status_color(&self, status: super::theme::SemanticStatus) -> Option<Color> {
        let style = self.style();
        match status {
            super::theme::SemanticStatus::Normal => None,
            super::theme::SemanticStatus::Success => Some(style.success),
            super::theme::SemanticStatus::Warning => Some(style.warning),
            super::theme::SemanticStatus::Error => Some(style.error),
        }
    }

    fn status_border(&self, color: Color, border: Border) -> Border {
        let width = if border.width > 0.0 {
            border.width
        } else {
            self.style().border.width.max(1.0)
        };
        Border { width, color }
    }

    /// Soft ring around an invalid control: a fifth of the color on light themes,
    /// two fifths on dark ones and while focused, so focus stays visible.
    fn status_ring(&self, color: Color, focus: bool) -> Border {
        let background = self.style().background.linear();
        let dark = background[0] * 0.2126 + background[1] * 0.7152 + background[2] * 0.0722 < 0.18;
        let strength = if focus || dark { 0.4 } else { 0.2 };
        let mut ring = self.style().focus_border;
        ring.width = 2.0;
        ring.color = alpha(color, strength);
        ring
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

pub fn resolve_control(
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
