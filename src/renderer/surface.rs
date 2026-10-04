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

impl Renderer {
    /// Composite this window with the desktop through the alpha of its clear color and
    /// drawing, where the surface supports premultiplied alpha. Without support the window
    /// stays opaque. Windows are opaque by default; the desktop runner enables this for
    /// windows created with a transparent attribute.
    pub fn set_transparent(&mut self, transparent: bool) {
        let modes = self.surface.get_capabilities(&self.adapter).alpha_modes;
        let mode = if transparent {
            modes
                .iter()
                .copied()
                .find(|mode| *mode == wgpu::CompositeAlphaMode::PreMultiplied)
        } else {
            modes.first().copied()
        };
        if let Some(mode) = mode.filter(|mode| *mode != self.config.alpha_mode) {
            self.config.alpha_mode = mode;
            if self.physical_size.width > 0 && self.physical_size.height > 0 {
                self.configure();
            }
        }
    }

    /// The clear color as the surface expects it: premultiplied when the window composites.
    pub(super) fn surface_clear(&self, clear: crate::Color) -> crate::Color {
        if self.config.alpha_mode != wgpu::CompositeAlphaMode::PreMultiplied {
            return clear;
        }
        let [r, g, b, a] = clear.0;
        let scale = |c: u8| (u16::from(c) * u16::from(a) / 255) as u8;
        crate::Color([scale(r), scale(g), scale(b), a])
    }
}
