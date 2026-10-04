//! Surface resizing.

use super::{RenderError, Renderer};
use winit::dpi::PhysicalSize;

impl Renderer {
    /// Reconfigure after native resize. Zero-sized windows suspend presentation.
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), RenderError> {
        self.physical_size = size;
        self.blur = None;
        if size.width == 0 || size.height == 0 {
            return Ok(());
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        if size.width > limit || size.height > limit {
            return Err(RenderError::InvalidDrawData(
                "viewport exceeds the GPU texture limit",
            ));
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.configure();
        Ok(())
    }

    pub(super) fn configure(&mut self) {
        self.surface.configure(&self.device, &self.config);
    }
}
