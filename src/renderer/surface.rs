//! Surface resizing and multisample attachments.

use super::{RenderError, Renderer};
use winit::dpi::PhysicalSize;

impl Renderer {
    /// Reconfigure after native resize. Zero-sized windows suspend presentation.
    pub fn resize(&mut self, size: PhysicalSize<u32>) -> Result<(), RenderError> {
        self.physical_size = size;
        self.blur = None;
        if size.width == 0 || size.height == 0 {
            self.msaa_view = None;
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
        self.msaa_view = (self.sample_count > 1).then(|| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("zaxis MSAA attachment"),
                    size: wgpu::Extent3d {
                        width: self.config.width,
                        height: self.config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: self.sample_count,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.attachment_format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });
    }
}
