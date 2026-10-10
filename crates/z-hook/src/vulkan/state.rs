//! What the layer knows about the host's Vulkan objects, by handle.

use super::{
    ffi::{Gdpa, Gipa, SetDeviceLoaderData},
    gpu::GpuSlot,
};
use ash::vk;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use zaxis::wgpu;

pub(super) struct InstanceData {
    pub handle: vk::Instance,
    pub next_gipa: Gipa,
    /// Dispatch to the layers below this one: calls made through it never come back here.
    pub raw: ash::Instance,
    pub entry: ash::Entry,
    /// The version the instance was created with (raised to 1.1 when the host asked for 1.0).
    pub api_version: u32,
    pub extensions: HashSet<String>,
    /// The wgpu instance over this Vulkan instance, made when the first device needs it.
    pub wgpu: Mutex<Option<Arc<wgpu::Instance>>>,
}

pub(super) struct DeviceData {
    pub handle: vk::Device,
    pub physical: vk::PhysicalDevice,
    pub instance: Arc<InstanceData>,
    pub raw: ash::Device,
    /// The swapchain functions, resolved through the next layer's `vkGetDeviceProcAddr`: asking
    /// the loader's instance-level one before the loader has finished creating the device
    /// jumps through a dispatch table that is not set yet.
    pub swapchain: ash::khr::swapchain::DeviceFn,
    pub next_gdpa: Gdpa,
    pub set_loader_data: Option<SetDeviceLoaderData>,
    pub next_allocate_command_buffers: vk::PFN_vkAllocateCommandBuffers,
    /// Present only when the layer could add what wgpu needs to the host's device.
    pub interop: Option<Interop>,
    pub gpu: Mutex<GpuSlot>,
    /// The thread creating `gpu`, and whether the device is going away (it must stop and be
    /// joined before the device is destroyed).
    pub worker: Mutex<Option<std::thread::JoinHandle<()>>>,
    pub closing: std::sync::atomic::AtomicBool,
    /// Queue handle to (family, index), filled as the host fetches its queues.
    pub queues: Mutex<HashMap<u64, (u32, u32)>>,
}

/// What `vkCreateDevice` added for wgpu, and what the device can do for the overlay.
pub(super) struct Interop {
    /// `VK_KHR_swapchain_mutable_format` (with its dependencies) is enabled: swapchain images
    /// may be viewed in their sRGB sibling format.
    pub mutable_format: bool,
    /// The extensions wgpu was told are enabled.
    pub extensions: Vec<&'static std::ffi::CStr>,
    pub family_flags: Vec<vk::QueueFlags>,
}

pub(super) struct SwapchainData {
    #[cfg_attr(
        not(all(feature = "x11", unix, not(target_os = "macos"))),
        allow(dead_code)
    )]
    pub handle: vk::SwapchainKHR,
    pub surface: vk::SurfaceKHR,
    pub device: Arc<DeviceData>,
    pub format: vk::Format,
    pub extent: vk::Extent2D,
    pub usage: vk::ImageUsageFlags,
    pub images: Vec<vk::Image>,
    /// One per image: signaled by the overlay's submit, waited on by the present.
    pub semaphores: Mutex<Vec<vk::Semaphore>>,
}

#[derive(Default)]
pub(super) struct Registry {
    pub instances: HashMap<u64, Arc<InstanceData>>,
    pub devices: HashMap<u64, Arc<DeviceData>>,
    pub queues: HashMap<u64, Arc<DeviceData>>,
    pub swapchains: HashMap<u64, Arc<SwapchainData>>,
    pub surfaces: HashMap<u64, super::surface::HostWindow>,
    /// Input readers of the target swapchain's window, by swapchain.
    #[cfg(all(feature = "x11", unix, not(target_os = "macos")))]
    pub inputs: HashMap<u64, super::x11::X11Input>,
    /// The swapchain the overlay draws on.
    pub target: Option<u64>,
}

static REGISTRY: Mutex<Option<Registry>> = Mutex::new(None);

/// Run `f` on the registry. Never call out to Vulkan or to the overlay inside `f`.
pub(super) fn with<R>(f: impl FnOnce(&mut Registry) -> R) -> R {
    let mut guard: MutexGuard<'_, Option<Registry>> =
        REGISTRY.lock().unwrap_or_else(PoisonError::into_inner);
    f(guard.get_or_insert_with(Registry::default))
}

impl SwapchainData {
    pub fn area(&self) -> u64 {
        u64::from(self.extent.width) * u64::from(self.extent.height)
    }
}
