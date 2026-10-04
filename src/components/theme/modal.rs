use super::{SurfaceStyle, TypographyRole};
use crate::{Color, Padding};

/// Modal dialogs and sheets. Missing properties inherit from the theme tokens;
/// geometry is in logical pixels. The overlay dims everything under the surface.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModalStyle {
    /// Dimming color of the overlay, including its alpha.
    pub overlay: Option<Color>,
    /// Backdrop blur sigma under the overlay (0 disables).
    pub overlay_blur: Option<f32>,
    /// Surface fill, border, shadow and corner radius.
    pub surface: SurfaceStyle,
    pub padding: Option<Padding>,
    /// Space between header, body and actions.
    pub gap: Option<f32>,
    /// Gap between a title and its description, and between actions.
    pub spacing: Option<f32>,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    /// Distance kept between the surface and the viewport edges.
    pub margin: Option<f32>,
    /// Scale and vertical offset of the hidden pose while entering or leaving.
    pub enter_scale: Option<f32>,
    pub enter_offset: Option<f32>,
    pub title: Option<TypographyRole>,
    pub description: Option<TypographyRole>,
    pub close_size: Option<f32>,
}

impl ModalStyle {
    pub(crate) fn merge(&mut self, rhs: Self) {
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f; })* }; }
        set!(
            overlay,
            overlay_blur,
            padding,
            gap,
            spacing,
            min_width,
            max_width,
            margin,
            enter_scale,
            enter_offset,
            title,
            description,
            close_size
        );
        self.surface.merge(rhs.surface);
    }
}
