//! The Vulkan [`PresentBackend`](crate::PresentBackend): wraps the swapchain image about to be
//! presented as a wgpu texture and draws the interface into it.

use super::{
    gpu::{format_plan, Gpu, GpuSlot},
    state::SwapchainData,
};
use crate::backend::{BackendError, PresentBackend, SurfaceInfo};
use ash::vk;
use std::sync::{Arc, PoisonError};
use wgpu::hal::{api::Vulkan, CommandEncoder as _};
use zaxis::wgpu;
use zaxis::{DrawData, EmbedLoad, EmbedOptions};

/// One present of the host on one swapchain.
pub(super) struct VulkanBackend<'a> {
    pub swapchain: &'a Arc<SwapchainData>,
    pub image_index: u32,
    pub family: u32,
    pub index: u32,
    /// The semaphores the host's present waits on; the overlay's submit waits on them instead.
    pub waits: &'a [vk::Semaphore],
    pub output: crate::OutputColor,
    /// Set once the overlay's submit consumed `waits`: the present must wait on this instead.
    pub signal: Option<vk::Semaphore>,
    gpu: Option<Arc<Gpu>>,
}

impl<'a> VulkanBackend<'a> {
    pub fn new(
        swapchain: &'a Arc<SwapchainData>,
        image_index: u32,
        (family, index): (u32, u32),
        waits: &'a [vk::Semaphore],
        output: crate::OutputColor,
    ) -> Self {
        Self {
            swapchain,
            image_index,
            family,
            index,
            waits,
            output,
            signal: None,
            gpu: None,
        }
    }

    fn options(&self) -> Result<EmbedOptions, BackendError> {
        let plan =
            format_plan(self.swapchain.format, self.output).map_err(BackendError::Unsupported)?;
        Ok(EmbedOptions::new(plan.texture))
    }
}

impl PresentBackend for VulkanBackend<'_> {
    fn begin(&mut self) -> Result<SurfaceInfo, BackendError> {
        let device = &self.swapchain.device;
        let Some(interop) = &device.interop else {
            return Err(BackendError::Unsupported(
                "the host's device was created before the overlay was installed".into(),
            ));
        };
        let graphics = interop
            .family_flags
            .get(self.family as usize)
            .is_some_and(|f| f.contains(vk::QueueFlags::GRAPHICS));
        if !graphics {
            return Err(BackendError::Unsupported(
                "the host presents on a queue family without graphics".into(),
            ));
        }
        let options = self.options()?;
        let slot = device.gpu.try_lock().map_err(|_| BackendError::NotReady)?;
        match &*slot {
            GpuSlot::Idle => {
                drop(slot);
                device.start_gpu(self.family, self.index, options);
                return Err(BackendError::NotReady);
            }
            GpuSlot::Starting => return Err(BackendError::NotReady),
            GpuSlot::Failed(reason) => return Err(BackendError::Unsupported(reason.clone())),
            GpuSlot::Ready(gpu) => {
                if gpu.family != self.family {
                    return Err(BackendError::Unsupported(
                        "the host presents on several queue families".into(),
                    ));
                }
                self.gpu = Some(Arc::clone(gpu));
            }
        }
        drop(slot);
        let extent = self.swapchain.extent;
        let scale = std::env::var("ZAXIS_HOOK_SCALE")
            .ok()
            .and_then(|value| value.parse().ok());
        Ok(SurfaceInfo {
            size: [extent.width, extent.height],
            scale_factor: scale,
        })
    }

    fn render(&mut self, data: &DrawData) -> Result<(), BackendError> {
        if data.indices.is_empty() {
            return Ok(());
        }
        let gpu = self.gpu.clone().ok_or(BackendError::NotReady)?;
        let plan =
            format_plan(self.swapchain.format, self.output).map_err(BackendError::Unsupported)?;
        let options = EmbedOptions::new(plan.texture);
        let image = *self
            .swapchain
            .images
            .get(self.image_index as usize)
            .ok_or_else(|| BackendError::Lost("image index out of range".into()))?;
        let semaphore = *self
            .swapchain
            .semaphores
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(self.image_index as usize)
            .ok_or_else(|| BackendError::Lost("no semaphore for the image".into()))?;
        {
            let mut renderer = gpu.renderer.lock().unwrap_or_else(PoisonError::into_inner);
            if *renderer.options() != options {
                renderer
                    .set_options(options)
                    .map_err(|e| BackendError::Unsupported(e.to_string()))?;
            }
        }
        let extent = wgpu::Extent3d {
            width: self.swapchain.extent.width,
            height: self.swapchain.extent.height,
            depth_or_array_layers: 1,
        };
        let view_formats: &[wgpu::TextureFormat] = if plan.mutable_format {
            &[plan.view]
        } else {
            &[]
        };
        let mut usage = wgpu::TextureUsages::RENDER_ATTACHMENT;
        let mut hal_usage = wgpu::TextureUses::COLOR_TARGET;
        if self
            .swapchain
            .usage
            .contains(vk::ImageUsageFlags::TRANSFER_SRC)
        {
            usage |= wgpu::TextureUsages::COPY_SRC;
            hal_usage |= wgpu::TextureUses::COPY_SRC;
        }
        let hal_descriptor = wgpu::hal::TextureDescriptor {
            label: Some("host swapchain image"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: plan.texture,
            usage: hal_usage,
            memory_flags: wgpu::hal::MemoryFlags::empty(),
            view_formats: view_formats.to_vec(),
        };
        // SAFETY: the device is a Vulkan one.
        let hal_device = unsafe { gpu.device.as_hal::<Vulkan>() }
            .ok_or_else(|| BackendError::Unsupported("not a Vulkan device".into()))?;
        // SAFETY: `image` is a swapchain image of this device made with the usage and view
        // formats declared; the host keeps owning it, so the drop callback does nothing.
        let hal_texture = unsafe {
            hal_device.texture_from_raw(
                image,
                &hal_descriptor,
                Some(Box::new(|| {})),
                wgpu::hal::vulkan::TextureMemory::External,
            )
        };
        drop(hal_device);
        let descriptor = wgpu::TextureDescriptor {
            label: Some("host swapchain image"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: plan.texture,
            usage,
            view_formats,
        };
        // SAFETY: the image is initialized and in the present layout, which `PRESENT` names.
        let texture = unsafe {
            gpu.device.create_texture_from_hal::<Vulkan>(
                hal_texture,
                &descriptor,
                wgpu::TextureUses::PRESENT,
            )
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(plan.view),
            ..Default::default()
        });
        let mut encoder = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("z-hook"),
            });
        gpu.renderer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .render_to(&mut encoder, &view, None, EmbedLoad::Keep, data)
            .map_err(|error| BackendError::Frame(error.to_string()))?;
        // Pin the texture's state: whichever way the frame ended, it is a color target now.
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("z-hook present state"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        let dump = super::dump::request(&gpu.device, plan.texture, [extent.width, extent.height])
            .filter(|_| usage.contains(wgpu::TextureUsages::COPY_SRC));
        // The state the texture is in when the frame's own commands end.
        let mut final_state = wgpu::TextureUses::COLOR_TARGET;
        if let Some(dump) = &dump {
            dump.copy(&mut encoder, &texture);
            final_state = wgpu::TextureUses::COPY_SRC;
        }
        // The state change back to the present layout is recorded on an encoder of its own,
        // through the raw API: wgpu does not allow mixing the two on one encoder.
        let mut present = gpu
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("z-hook present"),
            });
        // SAFETY: the encoder is a Vulkan one, fresh, and not touched while the closure records.
        unsafe {
            present.as_hal_mut::<Vulkan, _, _>(|raw| {
                let (Some(raw), Some(hal_texture)) = (raw, texture.as_hal::<Vulkan>()) else {
                    return;
                };
                raw.transition_textures(std::iter::once(wgpu::hal::TextureBarrier {
                    texture: &*hal_texture,
                    range: wgpu::ImageSubresourceRange::default(),
                    usage: wgpu::hal::StateTransition {
                        from: final_state,
                        to: wgpu::TextureUses::PRESENT,
                    },
                }));
            });
        }
        let commands = [encoder.finish(), present.finish()];
        // SAFETY: the queue is a Vulkan one.
        let hal_queue = unsafe { gpu.queue.as_hal::<Vulkan>() }
            .ok_or_else(|| BackendError::Unsupported("not a Vulkan queue".into()))?;
        for wait in self.waits {
            hal_queue.add_wait_semaphore(*wait, None, vk::PipelineStageFlags::ALL_COMMANDS);
        }
        hal_queue.add_signal_semaphore(semaphore, None);
        drop(hal_queue);
        log::trace!(
            "z-hook: image {} waits {:?} signals {:?}",
            self.image_index,
            self.waits,
            semaphore
        );
        gpu.queue.submit(commands);
        self.signal = Some(semaphore);
        let _ = gpu.device.poll(wgpu::PollType::Poll);
        if let Some(dump) = dump {
            dump.write(&gpu.device);
        }
        Ok(())
    }

    fn end(&mut self) -> Result<(), BackendError> {
        self.gpu = None;
        Ok(())
    }
}
