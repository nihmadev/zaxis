use super::{HoverFill, HoverStyle, Response, Ui, WidgetState};
use crate::{
    context::Paint, Border, Color, CornerRadius, Gradient, Interpolate, Rect, Shadow, Shape,
};

/// Resolved hover properties share one track, including gradient/shadow fades.
/// Hit geometry and control values never move with hover.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Appearance {
    pub fill: Gradient,
    pub border: Border,
    pub text_color: Color,
    shadow: Shadow,
}
impl Interpolate for Appearance {
    fn interpolate(&self, to: &Self, t: f32) -> Self {
        Self {
            fill: self.fill.interpolate(&to.fill, t),
            border: self.border.interpolate(&to.border, t),
            text_color: self.text_color.interpolate(&to.text_color, t),
            shadow: self.shadow.interpolate(&to.shadow, t),
        }
    }
}
impl Appearance {
    pub fn new(fill: Color, border: Border, text_color: Color) -> Self {
        Self {
            fill: Gradient::new(fill, fill),
            border,
            text_color,
            shadow: Shadow {
                color: Color::TRANSPARENT,
                ..Shadow::default()
            },
        }
    }
    pub fn paint_shadow(self, rect: Rect, rounding: CornerRadius, paint: &mut Vec<Paint>) {
        if self.shadow.color.0[3] > 0 {
            paint.push(Paint::Shape(Shape::Shadow {
                rect,
                rounding,
                shadow: self.shadow,
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
                fill: style.backdrop_fill(self.fill.start, blur),
                border: self.border,
            }));
        } else {
            paint.push(Paint::Shape(Shape::Gradient {
                rect,
                rounding,
                gradient: Gradient {
                    start: style.backdrop_fill(self.fill.start, blur),
                    end: style.backdrop_fill(self.fill.end, blur),
                    ..self.fill
                },
            }));
            paint.push(Paint::Shape(Shape::Rect {
                rect,
                rounding,
                fill: Color::TRANSPARENT,
                border: self.border,
            }));
        }
    }
}

impl Ui<'_> {
    pub(super) fn animate_hover(
        &mut self,
        response: Response,
        preset: HoverStyle,
        mut normal: Appearance,
        hovered: Color,
    ) -> Appearance {
        // Use the same gradient direction in the normal/hover endpoints.
        if let Some(HoverFill::Gradient(gradient)) = preset.fill {
            normal.fill.direction = gradient.direction;
        }
        if let Some(shadow) = preset.shadow {
            normal.shadow = Shadow {
                color: Color::rgba(shadow.color.0[0], shadow.color.0[1], shadow.color.0[2], 0),
                ..shadow
            };
        }
        let mut target = normal;
        if response.state() == WidgetState::Hovered {
            target.fill = match preset.fill {
                Some(HoverFill::Theme) => Gradient {
                    start: hovered,
                    end: hovered,
                    ..normal.fill
                },
                Some(HoverFill::Solid(color)) => Gradient {
                    start: color,
                    end: color,
                    ..normal.fill
                },
                Some(HoverFill::Gradient(gradient)) => gradient,
                None => normal.fill,
            };
            if let Some(border) = preset.border.filter(|_| !response.focus_visible) {
                target.border = border;
            }
            if let Some(color) = preset.text_color {
                target.text_color = color;
            }
            if let Some(shadow) = preset.shadow {
                target.shadow = shadow;
            }
        }
        let motion = self.style().motion.hover.clone();
        self.transition_property(
            response.id.with("appearance"),
            response.rect,
            target,
            motion,
        )
    }
}
