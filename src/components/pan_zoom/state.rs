//! Application-owned camera. Content units never depend on the parent's cursor or DPI.
use crate::{context::invalid_value, Rect, Transform, Vec2};
use std::ops::RangeInclusive;

pub(crate) const MAX_POSITION: f32 = 1.0e6;
pub(crate) const MIN_SCALE: f32 = 0.0001;
pub(crate) const MAX_SCALE: f32 = 64.0;

/// `viewport_position = local_position * scale + translation`.
/// Translation is in logical viewport units, before ancestor transforms and DPI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanZoomState {
    pub scale: f32,
    pub translation: Vec2,
}

impl Default for PanZoomState {
    fn default() -> Self {
        Self {
            scale: 1.0,
            translation: Vec2::ZERO,
        }
    }
}

impl PanZoomState {
    /// Deterministically restore scale 1 and translation zero. `show` applies its limits.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn local_to_viewport(self, local: Vec2) -> Vec2 {
        self.transform(Vec2::ZERO).point(local)
    }

    pub fn viewport_to_local(self, point: Vec2) -> Vec2 {
        self.transform(Vec2::ZERO).inverse().point(point)
    }

    /// Set an absolute scale, keeping `anchor` (logical viewport coordinates) fixed.
    /// Returns whether the camera actually changed. Invalid arguments are diagnosed.
    #[track_caller]
    pub fn zoom_at(&mut self, scale: f32, anchor: Vec2, limits: RangeInclusive<f32>) -> bool {
        let old = *self;
        let (min, max) = limits_for(limits);
        self.normalize(min, max);
        if super::super::sanitize::positive("PanZoomState::zoom_at", scale).is_some()
            && valid_position(anchor)
        {
            let local = self.viewport_to_local(anchor);
            self.scale = scale.clamp(min, max);
            self.translation = anchor - local * self.scale;
            self.normalize(min, max);
        }
        *self != old
    }

    /// Center `content` inside `viewport_size`, with padding in viewport logical units.
    /// Empty content/viewports return false; invalid or excessive coordinates are rejected.
    #[track_caller]
    pub fn fit(
        &mut self,
        content: Rect,
        viewport_size: Vec2,
        padding: f32,
        limits: RangeInclusive<f32>,
    ) -> bool {
        let Some(size) = super::super::sanitize::size("PanZoomState::fit", viewport_size) else {
            return false;
        };
        let Some(padding) = super::super::sanitize::non_negative("PanZoomState::fit", padding)
        else {
            return false;
        };
        if !valid_rect(content) || content.is_empty() || size.min_element() <= 0.0 {
            return false;
        }
        let available = size - Vec2::splat(padding * 2.0);
        if available.min_element() <= 0.0 {
            return false;
        }
        let (min, max) = limits_for(limits);
        let scale = (available / content.size()).min_element().clamp(min, max);
        let old = *self;
        self.scale = scale;
        self.translation = size * 0.5 - content.center() * scale;
        self.normalize(min, max);
        *self != old
    }

    pub(crate) fn transform(self, origin: Vec2) -> Transform {
        Transform {
            scale: self.scale,
            translation: origin + self.translation,
            angle: 0.0,
        }
    }

    #[track_caller]
    pub(crate) fn normalize(&mut self, min: f32, max: f32) {
        let old = *self;
        self.scale = if self.scale.is_finite() && self.scale > 0.0 {
            self.scale.clamp(min, max)
        } else {
            1.0_f32.clamp(min, max)
        };
        self.translation = if self.translation.is_finite() {
            self.translation
                .clamp(Vec2::splat(-MAX_POSITION), Vec2::splat(MAX_POSITION))
        } else {
            Vec2::ZERO
        };
        if *self != old {
            invalid_value(
                "PanZoom camera",
                "invalid/out-of-range camera normalized".into(),
            );
        }
    }
}

#[track_caller]
pub(crate) fn limits_for(limits: RangeInclusive<f32>) -> (f32, f32) {
    let (min, max) = (*limits.start(), *limits.end());
    if min.is_finite() && max.is_finite() && min >= MIN_SCALE && max <= MAX_SCALE && min <= max {
        (min, max)
    } else {
        invalid_value(
            "PanZoom scale limits",
            "expected 0.0001 <= min <= max <= 64; using 0.1..=8".into(),
        );
        (0.1, 8.0)
    }
}

#[track_caller]
pub(crate) fn valid_position(point: Vec2) -> bool {
    if point.is_finite() && point.abs().max_element() <= MAX_POSITION {
        true
    } else {
        invalid_value(
            "PanZoom position",
            "expected finite coordinates within +/-1000000; ignored".into(),
        );
        false
    }
}

#[track_caller]
pub(crate) fn valid_rect(rect: Rect) -> bool {
    super::super::sanitize::positioned_rect("PanZoom content", rect).is_some()
}
