//! What a modal draws with: the few theme values it reads, resolved once per pass
//! with the modal's own style into the surface appearance, spacing and placement
//! limits. Pure functions of the theme, the style and the viewport size.
use super::{
    geometry::{Metrics, ModalAnchor},
    overlay::Tint,
};
use crate::{
    components::{appearance::Appearance, theme::ModalStyle, ScrollStyle, Style},
    Color, Padding, Rect,
};

/// The few theme values the modal reads.
pub(super) struct Look {
    pub(super) modal: ModalStyle,
    pub(super) presence: crate::TweenOptions,
    pub(super) window_fill: Color,
    pub(super) border: crate::Border,
    pub(super) text_color: Color,
    pub(super) rounding: crate::CornerRadius,
    pub(super) blur_radius: f32,
    pub(super) elevation: crate::Shadow,
    pub(super) opacity: f32,
    pub(super) window_padding: Padding,
    pub(super) spacing: f32,
    pub(super) scroll: ScrollStyle,
    pub(super) control_height: f32,
}

/// The theme and the modal's style merged into the values of this pass.
pub(super) struct Resolved {
    pub(super) surface: Appearance,
    pub(super) spacing: Spacing,
    pub(super) min_width: f32,
    pub(super) max_width: f32,
    pub(super) overlay: Tint,
    /// Scale and vertical offset of the hidden pose.
    pub(super) enter_scale: f32,
    pub(super) enter_offset: f32,
    pub(super) close_size: f32,
    pub(super) scroll: ScrollStyle,
}

/// Room inside and around the surface.
#[derive(Clone, Copy)]
pub(super) struct Spacing {
    pub(super) padding: Padding,
    /// Between header, body and actions.
    pub(super) gap: f32,
    /// Between the surface and the viewport edges.
    pub(super) margin: f32,
}

impl Look {
    pub(super) fn of(style: &Style) -> Self {
        Self {
            modal: style.modal,
            presence: style.motion.presence.clone(),
            window_fill: style.window_fill,
            border: style.border,
            text_color: style.text_color,
            rounding: style.rounding,
            blur_radius: style.blur_radius,
            elevation: style.elevation,
            opacity: style.opacity,
            window_padding: style.window_padding,
            spacing: style.spacing,
            scroll: style.scroll,
            control_height: style.control_height,
        }
    }

    /// Merge `style` over the theme's modal style for a `viewport`.
    pub(super) fn resolve(&self, style: ModalStyle, viewport: Rect) -> Resolved {
        let mut component = self.modal;
        component.merge(style);
        let mut surface = Appearance::new(self.window_fill, self.border, self.text_color);
        surface.rounding = self.rounding;
        surface.blur = self.blur_radius;
        surface.shadow = self.elevation;
        surface.opacity = self.opacity;
        surface.apply(component.surface);
        let spacing = Spacing {
            padding: component.padding.unwrap_or(self.window_padding),
            gap: component.gap.unwrap_or(self.spacing).max(0.0),
            margin: component.margin.unwrap_or(16.0),
        };
        Resolved {
            surface,
            spacing: if compact(viewport) {
                spacing.tight()
            } else {
                spacing
            },
            min_width: component.min_width.unwrap_or(280.0),
            max_width: component.max_width.unwrap_or(520.0),
            overlay: Tint {
                color: component.overlay.unwrap_or(Color::rgba(0, 0, 0, 140)),
                blur: component.overlay_blur.unwrap_or(0.0),
            },
            enter_scale: component.enter_scale.unwrap_or(1.0).max(0.01),
            enter_offset: component.enter_offset.unwrap_or(0.0),
            close_size: component.close_size.unwrap_or(self.control_height - 4.0),
            scroll: self.scroll,
        }
    }
}

impl Resolved {
    /// Placement inputs of the geometry: the modal's options within these limits.
    pub(super) fn metrics(
        &self,
        anchor: ModalAnchor,
        width: Option<f32>,
        max_height: Option<f32>,
    ) -> Metrics {
        Metrics {
            anchor,
            width,
            max_height,
            min_width: self.min_width,
            max_width: self.max_width,
            margin: self.spacing.margin,
            padding: self.spacing.padding,
            gap: self.spacing.gap,
        }
    }
}

/// Very small windows trade breathing room for keeping everything on screen.
fn compact(viewport: Rect) -> bool {
    viewport.size().y < 320.0 || viewport.size().x < 360.0
}

impl Spacing {
    fn tight(self) -> Self {
        let tight = |v: f32, max: f32| v.min(max);
        let padding = self.padding;
        Self {
            padding: Padding {
                left: tight(padding.left, 12.0),
                right: tight(padding.right, 12.0),
                top: tight(padding.top, 12.0),
                bottom: tight(padding.bottom, 12.0),
            },
            gap: tight(self.gap, 8.0),
            margin: tight(self.margin, 4.0),
        }
    }
}
