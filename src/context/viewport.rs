//! Physical viewport sizing and DPI tracking.

use super::Context;
use crate::{Rect, Vec2};
use winit::dpi::PhysicalSize;

impl Context {
    pub fn viewport(&self) -> Rect {
        Rect::from_min_size(Vec2::ZERO, self.logical_size)
    }
    pub fn scale_factor(&self) -> f32 {
        self.scale
    }
    /// Update physical viewport size and DPI. Resize and DPI changes invalidate layout.
    pub fn set_viewport(&mut self, size: PhysicalSize<u32>, scale_factor: f64) {
        let scale = if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor as f32
        } else {
            1.0
        };
        let logical_size = Vec2::new(size.width as f32, size.height as f32) / scale;
        if self.logical_size != logical_size || self.scale != scale {
            self.logical_size = logical_size;
            self.scale = scale;
            self.request_repaint();
        }
    }
}
