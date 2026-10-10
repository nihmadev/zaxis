//! One host frame: the 3D scene, then the interface in the chosen mode.

use crate::{
    gpu::Gpu,
    interface::{Interface, Mode},
    scene::Scene,
    targets::Targets,
};
use std::error::Error;

/// Draw the scene (already updated) into `view` (a single-sample view of the host's target) and the interface
/// over it, if there is one, and submit. The host owns the encoder, the passes and the
/// submission.
pub fn draw(
    gpu: &Gpu,
    scene: &Scene,
    targets: &Targets,
    mut interface: Option<&mut Interface>,
    view: &wgpu::TextureView,
) -> Result<(), Box<dyn Error>> {
    let in_pass = interface.as_ref().is_some_and(|i| i.mode == Mode::Pass);
    if let (true, Some(interface)) = (in_pass, interface.as_deref_mut()) {
        interface.prepare(targets.size)?;
    }
    let mut encoder = gpu.device.create_command_encoder(&Default::default());
    {
        let (attachment, resolve) = match &targets.msaa {
            Some(msaa) => (msaa, Some(view)),
            None => (view, None),
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("host scene"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: attachment,
                depth_slice: None,
                resolve_target: resolve,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.02,
                        g: 0.04,
                        b: 0.1,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &targets.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        scene.draw(&mut pass);
        if let (true, Some(interface)) = (in_pass, interface.as_deref()) {
            interface.renderer.record(&mut pass)?;
        }
    }
    if let (false, Some(interface)) = (in_pass, interface) {
        interface.render_to(&mut encoder, view)?;
    }
    gpu.queue.submit([encoder.finish()]);
    Ok(())
}
