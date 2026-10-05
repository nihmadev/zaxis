use crate::{components::rich_text::Underline, Border, Color, Interpolate, Style};

/// Colors and decoration of links. Every field left `None` is derived from the theme: the
/// resting color is the accent, hover and press shift it away from the background, a visited
/// link is the accent muted toward the secondary text color, a disabled one the disabled text.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HyperlinkStyle {
    pub color: Option<Color>,
    pub hovered: Option<Color>,
    pub pressed: Option<Color>,
    pub visited: Option<Color>,
    pub disabled: Option<Color>,
    /// Default underline policy; a link's own policy wins.
    pub underline: Option<Underline>,
    /// Underline thickness in logical pixels; by default it follows the font size and
    /// is at least one physical pixel.
    pub thickness: Option<f32>,
    /// Ring around the link while it has keyboard focus; defaults to the theme's focus border.
    pub focus: Option<Border>,
}

/// The resolved colors of one link in each state.
#[derive(Clone, Copy, Debug)]
pub(crate) struct LinkColors {
    pub normal: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub visited: Color,
    pub disabled: Color,
}

fn luma(color: Color) -> f32 {
    let [r, g, b, _] = color.0;
    0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b)
}

impl HyperlinkStyle {
    pub(crate) fn resolve(&self, style: &Style) -> LinkColors {
        let accent = style.accent;
        let light = luma(style.window_fill) > 140.0;
        let toward = |target: Color, t: f32| accent.interpolate(&target, t);
        LinkColors {
            normal: self.color.unwrap_or(accent),
            hovered: self.hovered.unwrap_or_else(|| {
                if light {
                    toward(Color::BLACK, 0.2)
                } else {
                    toward(Color::WHITE, 0.28)
                }
            }),
            pressed: self
                .pressed
                .unwrap_or_else(|| toward(Color::BLACK, if light { 0.42 } else { 0.32 })),
            visited: self
                .visited
                .unwrap_or_else(|| accent.interpolate(&style.muted_text, 0.55)),
            disabled: self.disabled.unwrap_or(style.disabled_text),
        }
    }

    /// Underline thickness for text of `size`, a whole number of physical pixels.
    pub(crate) fn thickness_for(&self, size: f32, scale: f32) -> f32 {
        let scale = if scale > 0.0 { scale } else { 1.0 };
        let wanted = self.thickness.unwrap_or(size * 0.07);
        (wanted * scale).round().max(1.0) / scale
    }
}
