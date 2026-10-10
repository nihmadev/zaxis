//! wgpu over the host's own Vulkan device: created off the render thread, used on it.

use super::{device::wgpu_instance, state::DeviceData};
use ash::vk;
use std::sync::{atomic::Ordering, Arc, Mutex, PoisonError};
use wgpu::hal::api::Vulkan;
use zaxis::wgpu;
use zaxis::{EmbedOptions, EmbeddedRenderer};

/// The overlay's GPU objects for one host device.
pub(super) struct Gpu {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub renderer: Mutex<EmbeddedRenderer>,
    /// The queue wgpu submits to: the one the host presents on.
    pub family: u32,
}

pub(super) enum GpuSlot {
    Idle,
    Starting,
    Ready(Arc<Gpu>),
    Failed(String),
}

impl DeviceData {
    /// Begin creating the wgpu device on the host's `queue` in the background; the first
    /// present that finds it ready draws.
    pub fn start_gpu(self: &Arc<Self>, family: u32, index: u32, options: EmbedOptions) {
        {
            let mut slot = self.gpu.lock().unwrap_or_else(PoisonError::into_inner);
            if !matches!(*slot, GpuSlot::Idle) {
                return;
            }
            *slot = GpuSlot::Starting;
        }
        let device = Arc::clone(self);
        let spawned = std::thread::Builder::new()
            .name("z-hook-init".into())
            .spawn(move || {
                if device.closing.load(Ordering::Acquire) {
                    return;
                }
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    create(&device, family, index, options)
                }));
                let next = match result {
                    Ok(Ok(gpu)) if device.closing.load(Ordering::Acquire) => {
                        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
                        return;
                    }
                    Ok(Ok(gpu)) => GpuSlot::Ready(Arc::new(gpu)),
                    Ok(Err(message)) => GpuSlot::Failed(message),
                    Err(payload) => GpuSlot::Failed(format!(
                        "panic: {}",
                        crate::driver::panic_message(&payload)
                    )),
                };
                *device.gpu.lock().unwrap_or_else(PoisonError::into_inner) = next;
            });
        match spawned {
            Ok(handle) => {
                *self.worker.lock().unwrap_or_else(PoisonError::into_inner) = Some(handle)
            }
            Err(error) => {
                *self.gpu.lock().unwrap_or_else(PoisonError::into_inner) =
                    GpuSlot::Failed(error.to_string())
            }
        }
    }

    /// Drop every wgpu object of this device. Called before the device is destroyed.
    pub fn release_gpu(&self) {
        self.closing.store(true, Ordering::Release);
        if let Some(worker) = self
            .worker
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
        {
            let _ = worker.join();
        }
        let previous = std::mem::replace(
            &mut *self.gpu.lock().unwrap_or_else(PoisonError::into_inner),
            GpuSlot::Failed("device destroyed".into()),
        );
        if let GpuSlot::Ready(gpu) = previous {
            let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
            drop(gpu);
        }
    }

    /// Wait for the overlay's work to finish: before a swapchain's images go away.
    pub fn wait_gpu(&self) {
        if let GpuSlot::Ready(gpu) = &*self.gpu.lock().unwrap_or_else(PoisonError::into_inner) {
            let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        }
    }
}

fn create(
    device: &Arc<DeviceData>,
    family: u32,
    index: u32,
    options: EmbedOptions,
) -> Result<Gpu, String> {
    let interop = device
        .interop
        .as_ref()
        .ok_or("the device was not prepared for the overlay")?;
    let instance = wgpu_instance(&device.instance).ok_or("no wgpu instance")?;
    // SAFETY: the instance is a Vulkan one.
    let hal = unsafe { instance.as_hal::<Vulkan>() }.ok_or("not a Vulkan instance")?;
    let exposed = hal
        .expose_adapter(device.physical)
        .ok_or("wgpu does not accept the physical device")?;
    let limits = exposed.capabilities.limits.clone();
    // SAFETY: the raw device was created from this physical device with `interop.extensions`
    // and the features wgpu derives for them (see `features::merge`), on a queue family that
    // has `index` queues; the drop callback keeps wgpu from destroying the host's device.
    let open = unsafe {
        exposed.adapter.device_from_raw(
            device.raw.clone(),
            Some(Box::new(|| {})),
            &interop.extensions,
            wgpu::Features::empty(),
            &limits,
            &wgpu::MemoryHints::default(),
            family,
            index,
        )
    }
    .map_err(|error| format!("device_from_raw: {error}"))?;
    // SAFETY: `exposed` came from this instance.
    let adapter = unsafe { instance.create_adapter_from_hal::<Vulkan>(exposed) };
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("z-hook"),
        required_features: wgpu::Features::empty(),
        required_limits: limits,
        ..Default::default()
    };
    // SAFETY: `open` was created from this adapter, with no features beyond those requested.
    let (wgpu_device, queue) = unsafe { adapter.create_device_from_hal(open, &descriptor) }
        .map_err(|error| error.to_string())?;
    let renderer =
        EmbeddedRenderer::new_with_adapter(wgpu_device.clone(), queue.clone(), &adapter, options)
            .map_err(|error| error.to_string())?;
    Ok(Gpu {
        device: wgpu_device,
        queue,
        renderer: Mutex::new(renderer),
        family,
    })
}

/// The wgpu format of a swapchain format, the format the interface renders through, and
/// whether that needs the image to be viewable in another format.
pub(super) struct FormatPlan {
    pub texture: wgpu::TextureFormat,
    pub view: wgpu::TextureFormat,
    pub mutable_format: bool,
    pub view_vk: vk::Format,
}

pub(super) fn format_plan(
    format: vk::Format,
    output: crate::OutputColor,
) -> Result<FormatPlan, String> {
    use crate::OutputColor::{Auto, Linear, Srgb};
    use wgpu::TextureFormat as T;
    let (texture, view_vk, float) = match format {
        vk::Format::B8G8R8A8_UNORM => (T::Bgra8Unorm, vk::Format::B8G8R8A8_SRGB, false),
        vk::Format::B8G8R8A8_SRGB => (T::Bgra8UnormSrgb, format, false),
        vk::Format::R8G8B8A8_UNORM => (T::Rgba8Unorm, vk::Format::R8G8B8A8_SRGB, false),
        vk::Format::R8G8B8A8_SRGB => (T::Rgba8UnormSrgb, format, false),
        vk::Format::R16G16B16A16_SFLOAT => (T::Rgba16Float, format, true),
        vk::Format::A2B10G10R10_UNORM_PACK32 => (T::Rgb10a2Unorm, format, false),
        other => return Err(format!("swapchain format {other:?} cannot be drawn on")),
    };
    match (output, float) {
        (Srgb, true) => return Err("a float backbuffer holds linear values, not sRGB".into()),
        (Linear, false) => return Err("only a float backbuffer can hold linear values".into()),
        (Auto | Srgb | Linear, _) => {}
    }
    let view = texture.add_srgb_suffix();
    Ok(FormatPlan {
        texture,
        view,
        mutable_format: view != texture,
        view_vk,
    })
}
