use super::theme::{ButtonStyle, ControlStyle, SurfaceStyle};
use super::Style;
use crate::{Border, Color};

/// Visual emphasis of a [`Button`](super::Button). Variants only change colors,
/// so layout, input and focus behavior are identical across them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Neutral control fill with a hairline border.
    #[default]
    Surface,
    /// Accent fill for the one primary action in a view.
    Solid,
    /// Translucent accent tint without a border.
    Soft,
    /// Accent hairline on a transparent background.
    Outline,
    /// No chrome until hovered; for toolbars and dense lists.
    Ghost,
}

fn with_alpha(mut color: Color, alpha: u8) -> Color {
    color.0[3] = alpha;
    color
}
fn fill(color: Color) -> SurfaceStyle {
    SurfaceStyle::fill(color)
}

impl ButtonVariant {
    /// Style patch resolved against the active palette.
    pub(crate) fn style(self, s: &Style) -> ButtonStyle {
        let none = Border::NONE;
        let clear = Color::TRANSPARENT;
        let tint = |a| with_alpha(s.focus_border.color, a);
        let neutral = |a| with_alpha(s.text_color, a);
        let surface = match self {
            Self::Surface => return ButtonStyle::default(),
            Self::Solid => ControlStyle {
                idle: SurfaceStyle {
                    border: Some(none),
                    foreground: Some(s.on_accent),
                    ..fill(s.accent)
                },
                hover: fill(s.accent.interpolate_toward(Color::WHITE, 0.14)),
                pressed: fill(s.accent.interpolate_toward(Color::BLACK, 0.18)),
                disabled: SurfaceStyle {
                    border: Some(s.border),
                    ..fill(s.button_fill)
                },
                ..Default::default()
            },
            Self::Soft => ControlStyle {
                idle: SurfaceStyle {
                    border: Some(none),
                    foreground: Some(s.focus_border.color),
                    ..fill(tint(40))
                },
                hover: fill(tint(60)),
                pressed: fill(tint(80)),
                disabled: SurfaceStyle {
                    border: Some(none),
                    ..fill(neutral(14))
                },
                ..Default::default()
            },
            Self::Outline => ControlStyle {
                idle: SurfaceStyle {
                    border: Some(Border::new(s.border.width, tint(150))),
                    foreground: Some(s.focus_border.color),
                    ..fill(clear)
                },
                hover: fill(tint(24)),
                pressed: fill(tint(44)),
                disabled: SurfaceStyle {
                    border: Some(s.border),
                    ..fill(clear)
                },
                ..Default::default()
            },
            Self::Ghost => ControlStyle {
                idle: SurfaceStyle {
                    border: Some(none),
                    ..fill(clear)
                },
                hover: fill(neutral(22)),
                pressed: fill(neutral(34)),
                disabled: SurfaceStyle {
                    border: Some(none),
                    ..fill(clear)
                },
                ..Default::default()
            },
        };
        ButtonStyle {
            surface,
            ..Default::default()
        }
    }
}

trait Toward {
    fn interpolate_toward(self, to: Color, t: f32) -> Color;
}
impl Toward for Color {
    fn interpolate_toward(self, to: Color, t: f32) -> Color {
        crate::Interpolate::interpolate(&self, &to, t)
    }
}
