//! The host's device. zaxis receives clones of the device and queue and nothing else.

use std::{error::Error, sync::Arc};

pub struct Gpu {
    /// Kept alive for as long as the device is used.
    pub _instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
}

impl Gpu {
    /// A device for `window`, or for off-screen work when there is none.
    pub fn new(
        window: Option<&Arc<winit::window::Window>>,
    ) -> Result<(Self, Option<wgpu::Surface<'static>>), Box<dyn Error>> {
        let mut descriptor = match window {
            Some(window) => wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
                Arc::clone(window),
            )),
            None => wgpu::InstanceDescriptor::new_without_display_handle_from_env(),
        };
        descriptor.backends = wgpu::Backends::from_env().unwrap_or_default();
        let instance = wgpu::Instance::new(descriptor);
        let surface = window
            .map(|window| instance.create_surface(Arc::clone(window)))
            .transpose()?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: surface.as_ref(),
            ..Default::default()
        }))?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("host device"),
                ..Default::default()
            }))?;
        Ok((
            Self {
                _instance: instance,
                adapter,
                device,
                queue,
            },
            surface,
        ))
    }

    /// The largest sample count of 1, 4 and 2 that the adapter renders `color` and `depth` with.
    pub fn samples(&self, color: wgpu::TextureFormat, depth: wgpu::TextureFormat) -> u32 {
        let supports = |format, count| {
            self.adapter
                .get_texture_format_features(format)
                .flags
                .sample_count_supported(count)
        };
        [4, 2]
            .into_iter()
            .find(|count| supports(color, *count) && supports(depth, *count))
            .unwrap_or(1)
    }
}
