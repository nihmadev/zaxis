//! Drag-and-drop appearance and timing. `None` inherits Style tokens.
use crate::{Border, Color, CornerRadius, Style, TweenOptions, Vec2};
use std::time::Duration;

/// Part of [`Style`]; override per source or target with the builders, locally
/// with `Ui::with_style`, or globally through the theme overrides.
/// Priority: builder call, then `Style::drag`, then the defaults noted below.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DragStyle {
    /// Pointer travel that turns a press into a drag, in logical pixels (5).
    pub threshold: Option<f32>,
    /// Hold time required before movement starts a drag (none).
    pub delay: Option<Duration>,
    /// Preview opacity (0.62) and its largest size, scaled down to fit (320x200).
    pub preview_opacity: Option<f32>,
    pub preview_max_size: Option<Vec2>,
    /// Source opacity while it is dragged (0.4); `hide_source` makes it invisible.
    pub source_opacity: Option<f32>,
    pub hide_source: Option<bool>,
    /// Highlight of an accepting target (accent at ~15% alpha) and its outline.
    pub target_fill: Option<Color>,
    pub target_border: Option<Border>,
    pub target_rounding: Option<CornerRadius>,
    /// Outline of a rejecting target (error color).
    pub reject_border: Option<Border>,
    /// Insertion line color (accent) and thickness (2).
    pub line_color: Option<Color>,
    pub line_thickness: Option<f32>,
    /// Autoscroll band inside a scrollable edge (32) and speeds in logical px/s (80..900).
    pub autoscroll_edge: Option<f32>,
    pub autoscroll_min_speed: Option<f32>,
    pub autoscroll_max_speed: Option<f32>,
    /// Highlight/line fade (`Style::motion.hover`) and preview return (`Style::motion.reorder`).
    pub motion: Option<TweenOptions>,
    pub return_motion: Option<TweenOptions>,
}

impl DragStyle {
    pub fn threshold(mut self, pixels: f32) -> Self {
        self.threshold = Some(length(pixels));
        self
    }
    pub fn delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }
    #[track_caller]
    pub fn preview_opacity(mut self, opacity: f32) -> Self {
        self.preview_opacity = Some(crate::components::sanitize::unit(
            "DragStyle::preview_opacity",
            opacity,
        ));
        self
    }
    pub fn preview_max_size(mut self, size: Vec2) -> Self {
        self.preview_max_size = Some(Vec2::new(length(size.x), length(size.y)));
        self
    }
    #[track_caller]
    pub fn source_opacity(mut self, opacity: f32) -> Self {
        self.source_opacity = Some(crate::components::sanitize::unit(
            "DragStyle::source_opacity",
            opacity,
        ));
        self
    }
    pub fn hide_source(mut self, hide: bool) -> Self {
        self.hide_source = Some(hide);
        self
    }
    pub fn target(
        mut self,
        fill: Color,
        border: Border,
        rounding: impl Into<CornerRadius>,
    ) -> Self {
        self.target_fill = Some(fill);
        self.target_border = Some(border);
        self.target_rounding = Some(rounding.into());
        self
    }
    pub fn reject_border(mut self, border: Border) -> Self {
        self.reject_border = Some(border);
        self
    }
    pub fn line(mut self, color: Color, thickness: f32) -> Self {
        self.line_color = Some(color);
        self.line_thickness = Some(length(thickness));
        self
    }
    pub fn autoscroll(mut self, edge: f32, min_speed: f32, max_speed: f32) -> Self {
        self.autoscroll_edge = Some(length(edge));
        self.autoscroll_min_speed = Some(length(min_speed));
        self.autoscroll_max_speed = Some(length(max_speed).max(min_speed));
        self
    }
    pub fn motion(mut self, motion: TweenOptions) -> Self {
        self.motion = Some(motion);
        self
    }
    pub fn return_motion(mut self, motion: TweenOptions) -> Self {
        self.return_motion = Some(motion);
        self
    }
    pub(crate) fn merge(&mut self, rhs: &Self) {
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f.clone(); })* }; }
        set!(
            threshold,
            delay,
            preview_opacity,
            preview_max_size,
            source_opacity,
            hide_source,
            target_fill,
            target_border,
            target_rounding,
            reject_border,
            line_color,
            line_thickness,
            autoscroll_edge,
            autoscroll_min_speed,
            autoscroll_max_speed,
            motion,
            return_motion
        );
    }
}

#[track_caller]
fn length(value: f32) -> f32 {
    crate::components::sanitize::length("drag size", value)
}

/// Every value resolved against the current [`Style`]; `Copy` so sessions can
/// keep it without holding the style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resolved {
    pub threshold: f32,
    pub delay: Duration,
    pub preview_opacity: f32,
    pub preview_max_size: Vec2,
    pub source_opacity: f32,
    pub edge: f32,
    pub min_speed: f32,
    pub max_speed: f32,
    pub frame_interval: Duration,
    pub reduced_motion: bool,
}

impl DragStyle {
    pub fn resolve(&self, style: &Style) -> Resolved {
        let local = &style.drag;
        let pick = |f: fn(&DragStyle) -> Option<f32>, default: f32| {
            f(self).or_else(|| f(local)).unwrap_or(default)
        };
        let source_opacity = if self.hide_source.or(local.hide_source).unwrap_or(false) {
            0.0
        } else {
            pick(|s| s.source_opacity, 0.4)
        };
        let min_speed = pick(|s| s.autoscroll_min_speed, 80.0);
        Resolved {
            threshold: pick(|s| s.threshold, 5.0),
            delay: self.delay.or(local.delay).unwrap_or(Duration::ZERO),
            preview_opacity: pick(|s| s.preview_opacity, 0.62),
            preview_max_size: self
                .preview_max_size
                .or(local.preview_max_size)
                .unwrap_or(Vec2::new(320.0, 200.0)),
            source_opacity,
            edge: pick(|s| s.autoscroll_edge, 32.0),
            min_speed,
            max_speed: pick(|s| s.autoscroll_max_speed, 900.0).max(min_speed),
            frame_interval: style.motion.frame_interval,
            reduced_motion: style.motion.reduced_motion,
        }
    }
    pub(crate) fn tween(&self, style: &Style) -> TweenOptions {
        let options = self
            .motion
            .clone()
            .or_else(|| style.drag.motion.clone())
            .unwrap_or_else(|| style.motion.hover.clone());
        if style.motion.reduced_motion {
            TweenOptions::new(Duration::ZERO)
        } else {
            options
        }
    }
}

/// Token-derived colors for the current style.
pub(crate) struct Paints {
    pub fill: Color,
    pub border: Border,
    pub rounding: CornerRadius,
    pub reject: Border,
    pub line: Color,
    pub thickness: f32,
}

impl DragStyle {
    pub(crate) fn paints(&self, style: &Style) -> Paints {
        let local = &style.drag;
        let accent = style.accent;
        let mut fill = accent;
        fill.0[3] = 40;
        Paints {
            fill: self.target_fill.or(local.target_fill).unwrap_or(fill),
            border: self
                .target_border
                .or(local.target_border)
                .unwrap_or(Border::new(1.5, accent)),
            rounding: self
                .target_rounding
                .or(local.target_rounding)
                .unwrap_or(style.rounding),
            reject: self
                .reject_border
                .or(local.reject_border)
                .unwrap_or(Border::new(1.5, style.error)),
            line: self.line_color.or(local.line_color).unwrap_or(accent),
            thickness: self.line_thickness.or(local.line_thickness).unwrap_or(2.0),
        }
    }
}
