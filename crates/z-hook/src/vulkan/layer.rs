//! The layer's entry points and dispatch by name.

use super::{
    device,
    ffi::{self, guarded, name_of, VoidFn},
    instance, present, state, surface, swapchain,
};
use crate::{Overlay, OverlayOptions};
use ash::vk::{self, Handle};
use std::{
    ffi::c_char,
    sync::{Mutex, PoisonError},
};
use zaxis::Context;

pub use ffi::NegotiateLayerInterface;

type BoxedUi = Box<dyn FnMut(&mut Context) + Send>;

/// What a layer's library provides the Vulkan loader: the options of the overlay and its
/// interface, made once.
pub trait LayerEntry {
    fn into_overlay(self) -> (OverlayOptions, BoxedUi);
}

impl<F, U> LayerEntry for F
where
    F: FnOnce() -> (OverlayOptions, U),
    U: FnMut(&mut Context) + Send + 'static,
{
    fn into_overlay(self) -> (OverlayOptions, BoxedUi) {
        let (options, ui) = self();
        (options, Box::new(ui))
    }
}

/// The overlay installed by the layer, kept for the life of the process.
static INSTALLED: Mutex<Option<Overlay>> = Mutex::new(None);

/// Negotiate with the Vulkan loader and install the overlay (once). Called by the symbol
/// [`vulkan_layer!`](crate::vulkan_layer) exports.
///
/// # Safety
/// `interface` must be null or the loader's `VkNegotiateLayerInterface`.
pub unsafe fn negotiate_layer(
    interface: *mut NegotiateLayerInterface,
    entry: impl LayerEntry,
) -> i32 {
    crate::log::init_from_env();
    let installed = guarded("layer setup", false, || {
        let mut slot = INSTALLED.lock().unwrap_or_else(PoisonError::into_inner);
        if slot.is_some() {
            return true;
        }
        let (mut options, ui) = entry.into_overlay();
        options.apis = options.apis.with(crate::Api::Vulkan);
        match Overlay::install(options, ui) {
            Ok(overlay) => {
                *slot = Some(overlay);
                true
            }
            Err(error) => {
                log::error!("z-hook: the layer could not install the overlay: {error}");
                false
            }
        }
    });
    if !installed {
        return vk::Result::ERROR_INITIALIZATION_FAILED.as_raw();
    }
    // SAFETY: the loader's structure, or null (handled).
    unsafe { ffi::negotiate(interface, get_instance_proc_addr, get_device_proc_addr) }.as_raw()
}

/// A hook as the generic function pointer the loader expects; it casts it back to the type
/// its name implies.
fn erased(function: *const ()) -> VoidFn {
    // SAFETY: function pointers and `*const ()` have the same size and representation.
    Some(unsafe { std::mem::transmute::<*const (), unsafe extern "system" fn()>(function) })
}

fn instance_hook(name: &str) -> VoidFn {
    {
        match name {
            "vkGetInstanceProcAddr" => erased(get_instance_proc_addr as *const ()),
            "vkCreateInstance" => erased(instance::create_instance as *const ()),
            "vkDestroyInstance" => erased(instance::destroy_instance as *const ()),
            "vkCreateDevice" => erased(device::create_device as *const ()),
            "vkCreateXcbSurfaceKHR" => erased(surface::create_xcb_surface as *const ()),
            "vkCreateXlibSurfaceKHR" => erased(surface::create_xlib_surface as *const ()),
            "vkCreateWaylandSurfaceKHR" => erased(surface::create_wayland_surface as *const ()),
            "vkDestroySurfaceKHR" => erased(surface::destroy_surface as *const ()),
            _ => device_hook(name),
        }
    }
}

fn device_hook(name: &str) -> VoidFn {
    {
        match name {
            "vkGetDeviceProcAddr" => erased(get_device_proc_addr as *const ()),
            "vkDestroyDevice" => erased(device::destroy_device as *const ()),
            "vkGetDeviceQueue" => erased(device::get_device_queue as *const ()),
            "vkGetDeviceQueue2" => erased(device::get_device_queue2 as *const ()),
            "vkCreateSwapchainKHR" => erased(swapchain::create_swapchain as *const ()),
            "vkDestroySwapchainKHR" => erased(swapchain::destroy_swapchain as *const ()),
            "vkQueuePresentKHR" => erased(present::queue_present as *const ()),
            _ => None,
        }
    }
}

unsafe extern "system" fn get_instance_proc_addr(
    instance: vk::Instance,
    p_name: *const c_char,
) -> VoidFn {
    // SAFETY: the loader passes a C string.
    let name = unsafe { name_of(p_name) };
    if instance == vk::Instance::null() {
        return match name {
            "vkGetInstanceProcAddr" | "vkCreateInstance" => instance_hook(name),
            _ => None,
        };
    }
    let next = state::with(|registry| {
        registry
            .instances
            .get(&instance.as_raw())
            .map(|data| data.next_gipa)
    });
    // Device-level functions fetched through the instance reach the same hooks, which find
    // their device by handle.
    instance_hook(name).or_else(|| {
        // SAFETY: the next layer resolves the name for this instance.
        next.and_then(|next| unsafe { next(instance, p_name) })
    })
}

unsafe extern "system" fn get_device_proc_addr(
    device: vk::Device,
    p_name: *const c_char,
) -> VoidFn {
    // SAFETY: the loader passes a C string.
    let name = unsafe { name_of(p_name) };
    let next = state::with(|registry| {
        registry
            .devices
            .get(&device.as_raw())
            .map(|data| data.next_gdpa)
    });
    let hooked = device_hook(name);
    match (hooked, next) {
        (Some(hook), Some(next)) => {
            // Only hook what the next layer provides (the extension may not be enabled).
            // SAFETY: the next layer resolves the name for this device.
            unsafe { next(device, p_name) }.map(|_| hook)
        }
        // SAFETY: as above.
        (None, Some(next)) => unsafe { next(device, p_name) },
        _ => None,
    }
}
