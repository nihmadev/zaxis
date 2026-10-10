use crate::{Border, Color, CornerRadius, SpringOptions, Style, SurfaceStyle, TabsStyle, Vec2};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DockPreset {
    #[default]
    Tiled,
    Flat,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DockMotion {
    #[default]
    Snappy,
    Smooth,
    Off,
}
impl DockMotion {
    /// Critical damping (no overshoot from rest); distance in logical px.
    pub fn spring(self) -> SpringOptions {
        SpringOptions::frequency(
            match self {
                Self::Snappy => 6.0,
                Self::Smooth => 4.5,
                Self::Off => 0.0,
            },
            1.0,
        )
        .thresholds(0.2, 0.8)
    }
}
/// Missing fields inherit theme tokens. Colors always come from the active Style.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DockStyle {
    pub preset: Option<DockPreset>,
    pub motion: Option<DockMotion>,
    pub gap: Option<f32>,
    pub panel_radius: Option<CornerRadius>,
    pub surface: SurfaceStyle,
    pub inactive_border: Option<Border>,
    pub ring_width: Option<f32>,
    pub ring_gap: Option<f32>,
    pub ring_active: Option<Color>,
    pub ring_inactive: Option<Color>,
    pub ring_glow: Option<f32>,
    /// Opt-in rotating two-color contour. Sleeps while unfocused or reduced motion.
    pub ring_gradient: Option<bool>,
    pub preview_color: Option<Color>,
    pub minimum: Option<Vec2>,
    pub padding: Option<f32>,
    pub tabs: TabsStyle,
    pub spring: Option<SpringOptions>,
}
#[derive(Clone, Copy)]
pub(super) struct Look {
    pub gap: f32,
    pub radius: CornerRadius,
    pub surface: super::super::appearance::Appearance,
    pub ring_width: f32,
    pub ring_gap: f32,
    pub active: Color,
    pub inactive: Color,
    pub glow: f32,
    pub gradient: bool,
    pub preview: Color,
    pub minimum: Vec2,
    pub padding: f32,
    pub tabs: TabsStyle,
    pub spring: SpringOptions,
    pub off: bool,
    pub preset: DockPreset,
}
impl Look {
    pub fn resolve(style: &Style, override_style: DockStyle) -> Self {
        let mut d = style.dock;
        d.merge(override_style);
        let flat = d.preset.unwrap_or_default() == DockPreset::Flat;
        let motion = d.motion.unwrap_or_default();
        let finite = |v: Option<f32>, fallback: f32| {
            v.filter(|n| n.is_finite() && *n >= 0.0).unwrap_or(fallback)
        };
        let mut tabs = style.tabs;
        tabs.merge(d.tabs);
        let radius = d.panel_radius.or(d.surface.rounding).unwrap_or(if flat {
            CornerRadius::ZERO
        } else {
            CornerRadius::all(10.0)
        });
        let radius = CornerRadius {
            top_left: finite(Some(radius.top_left), 10.0),
            top_right: finite(Some(radius.top_right), 10.0),
            bottom_left: finite(Some(radius.bottom_left), 10.0),
            bottom_right: finite(Some(radius.bottom_right), 10.0),
        };
        let spring = d
            .spring
            .filter(valid_spring)
            .unwrap_or_else(|| motion.spring());
        let mut surface = super::super::appearance::Appearance::new(
            style.window_fill,
            style.border,
            style.text_color,
        );
        surface.apply(d.surface);
        surface.rounding = radius;
        if let Some(border) = d.inactive_border {
            surface.border = border;
        }
        Self {
            gap: finite(d.gap, if flat { 1.0 } else { style.spacing }),
            radius,
            surface,
            ring_width: finite(d.ring_width, if flat { 1.0 } else { 2.0 }),
            ring_gap: finite(d.ring_gap, if flat { 0.0 } else { 1.0 }),
            active: d.ring_active.unwrap_or(style.accent),
            inactive: d.ring_inactive.unwrap_or(style.muted_text),
            glow: finite(d.ring_glow, if flat { 0.0 } else { 5.0 }),
            gradient: d.ring_gradient.unwrap_or(false),
            preview: d.preview_color.unwrap_or(style.accent),
            minimum: d
                .minimum
                .filter(|v| v.is_finite())
                .unwrap_or(Vec2::new(120.0, 80.0))
                .max(Vec2::ZERO),
            padding: finite(d.padding, style.spacing),
            tabs,
            spring,
            off: motion == DockMotion::Off || style.motion.reduced_motion,
            preset: d.preset.unwrap_or_default(),
        }
    }
}
fn valid_spring(s: &SpringOptions) -> bool {
    s.stiffness.is_finite()
        && s.stiffness >= 0.0
        && s.damping.is_finite()
        && s.damping > 0.0
        && s.mass.is_finite()
        && s.mass > 0.0
        && s.distance_threshold.is_finite()
        && s.distance_threshold > 0.0
        && s.velocity_threshold.is_finite()
        && s.velocity_threshold > 0.0
        && (s.stiffness / s.mass).is_finite()
        && (s.damping / s.mass).powi(2).is_finite()
}
