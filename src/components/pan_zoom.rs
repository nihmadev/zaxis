//! A camera and fixed viewport, built from the shared placement and input paths.
mod show;
pub(crate) mod state;
pub use state::PanZoomState;

use crate::{Context, Id, Rect, Response, Transform, Vec2};
use std::{hash::Hash, ops::RangeInclusive};

/// Which wheel events are reserved for zoom. Other wheel events retain scroll routing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZoomWheel {
    /// Ctrl on Windows/Linux, Cmd on macOS; Shift/Alt combinations are not reserved.
    #[default]
    Primary,
    /// Reserve unmodified vertical wheel movement.
    Unmodified,
    Disabled,
}

/// A viewport for application-defined content. Camera state belongs to the application.
pub struct PanZoom {
    source: Id,
    size: Vec2,
    limits: (f32, f32),
    wheel: ZoomWheel,
    pan: bool,
    enabled: bool,
    label: String,
}

impl PanZoom {
    #[track_caller]
    pub fn new(source: impl Hash, size: Vec2) -> Self {
        let size = super::sanitize::size("PanZoom::new", size).unwrap_or(Vec2::ZERO);
        Self {
            source: Id::new(source),
            size: size.min(Vec2::splat(state::MAX_POSITION)),
            limits: (0.1, 8.0),
            wheel: ZoomWheel::Primary,
            pan: true,
            enabled: true,
            label: "Pan and zoom".into(),
        }
    }
    #[track_caller]
    pub fn scale_limits(mut self, limits: RangeInclusive<f32>) -> Self {
        self.limits = state::limits_for(limits);
        self
    }
    pub fn zoom_wheel(mut self, wheel: ZoomWheel) -> Self {
        self.wheel = wheel;
        self
    }
    /// Primary drag of the background. Child controls and drag sources have priority.
    pub fn pan_drag(mut self, pan: bool) -> Self {
        self.pan = pan;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn accessible_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

/// Geometry is in content or parent-layout logical units as marked below.
/// Screen helpers include completed ancestor placement/visual transforms; use after `run`.
pub struct PanZoomOutput<R> {
    pub inner: R,
    pub response: Response,
    /// Allocation in the parent's layout coordinates, before ancestor placement.
    pub viewport: Rect,
    /// Full viewport mapped to content coordinates; conservative under ancestor clipping.
    pub visible: Rect,
    /// Includes external camera edits since the last published pass and normalization.
    pub changed: bool,
    /// Actual changes caused by routed input during this pass.
    pub panned: bool,
    pub zoomed: bool,
    pub camera: PanZoomState,
}

impl<R> PanZoomOutput<R> {
    /// Content → displayed logical viewport coordinates; physical pixels multiply by DPI.
    pub fn transform(&self, context: &Context) -> Transform {
        context
            .visual_transform(self.response.id)
            .compose(self.camera.transform(self.viewport.min))
    }
    pub fn local_to_screen(&self, context: &Context, local: Vec2) -> Vec2 {
        self.transform(context).point(local)
    }
    pub fn screen_to_local(&self, context: &Context, screen: Vec2) -> Vec2 {
        self.transform(context).inverse().point(screen)
    }
    pub fn displayed_viewport(&self, context: &Context) -> Rect {
        context.visual_rect(self.response.id, self.viewport)
    }
}
