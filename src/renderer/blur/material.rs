//! Backdrop of materials: the sharp copy and the blurred copy bound together as group 2.

use super::Target;
use crate::renderer::Renderer;

/// The bind group of a window's backdrop targets for materials that read the backdrop.
/// Rebuilt whenever either target is recreated.
pub(super) fn backdrop_group(
    renderer: &Renderer,
    backdrop: &Target,
    blurred: &Target,
) -> wgpu::BindGroup {
    let pipelines = renderer.materials.lock().expect("material pipelines mutex");
    renderer
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zaxis material backdrop"),
            layout: &pipelines.backdrop_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&backdrop.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&blurred.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&renderer.sampler),
                },
            ],
        })
}
