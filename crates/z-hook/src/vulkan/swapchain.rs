//! `vkCreateSwapchainKHR` and `vkDestroySwapchainKHR`: the images the overlay draws on must
//! be attachable, copyable and viewable in sRGB, which the host did not necessarily ask for.

use super::{
    ffi::guarded,
    gpu::format_plan,
    state::{self, DeviceData, SwapchainData},
};
use ash::vk::{self, Handle};
use std::{
    ptr,
    sync::{Arc, Mutex, PoisonError},
};

/// The host's create info with usage and view formats added, and what it keeps alive.
struct Modified {
    info: vk::SwapchainCreateInfoKHR<'static>,
    formats: Vec<vk::Format>,
    list: vk::ImageFormatListCreateInfo<'static>,
    usage: vk::ImageUsageFlags,
}

/// # Safety
/// `host` must be the host's valid create info for `device`.
unsafe fn modify(
    device: &Arc<DeviceData>,
    host: &vk::SwapchainCreateInfoKHR<'_>,
) -> Option<Box<Modified>> {
    let interop = device.interop.as_ref()?;
    let core = crate::registry::current()?;
    let plan = format_plan(host.image_format, core.options.output)
        .map_err(|reason| log::warn!("z-hook: not drawing on this swapchain: {reason}"))
        .ok()?;
    let surface = ash::khr::surface::Instance::new(&device.instance.entry, &device.instance.raw);
    // SAFETY: the surface and physical device are the host's, alive for this call.
    let capabilities =
        unsafe { surface.get_physical_device_surface_capabilities(device.physical, host.surface) }
            .ok()?;
    if !capabilities
        .supported_usage_flags
        .contains(vk::ImageUsageFlags::COLOR_ATTACHMENT)
    {
        return None;
    }
    let mutable_format = plan.mutable_format;
    if mutable_format && !interop.mutable_format {
        log::warn!(
            "z-hook: the device cannot view {:?} as sRGB (no VK_KHR_swapchain_mutable_format)",
            host.image_format
        );
        return None;
    }
    let mut info = *host;
    info.image_usage |= vk::ImageUsageFlags::COLOR_ATTACHMENT;
    if capabilities
        .supported_usage_flags
        .contains(vk::ImageUsageFlags::TRANSFER_SRC)
    {
        info.image_usage |= vk::ImageUsageFlags::TRANSFER_SRC;
    }
    // SAFETY: the host's chain is valid.
    let mut formats = vec![host.image_format];
    let mut list = vk::ImageFormatListCreateInfo::default();
    if mutable_format {
        info.flags |= vk::SwapchainCreateFlagsKHR::MUTABLE_FORMAT;
        // SAFETY: the host's chain is valid; an existing format list is widened in place.
        let existing = unsafe { find_format_list(host.p_next) };
        match existing {
            Some(existing) => {
                // SAFETY: the host's list is valid; its formats are read.
                unsafe {
                    formats = std::slice::from_raw_parts(
                        (*existing).p_view_formats,
                        (*existing).view_format_count as usize,
                    )
                    .to_vec();
                }
                if !formats.contains(&host.image_format) {
                    formats.push(host.image_format);
                }
                if !formats.contains(&plan.view_vk) {
                    formats.push(plan.view_vk);
                }
            }
            None => formats.push(plan.view_vk),
        }
        let usage = info.image_usage;
        // SAFETY: the lifetime is erased; the box owns what the info points to.
        let mut boxed = Box::new(Modified {
            info: unsafe {
                std::mem::transmute::<
                    vk::SwapchainCreateInfoKHR<'_>,
                    vk::SwapchainCreateInfoKHR<'static>,
                >(info)
            },
            formats,
            list,
            usage,
        });
        if let Some(existing) = existing {
            // SAFETY: widening the host's list to the merged formats, which `boxed` outlives.
            unsafe {
                (*existing).view_format_count = boxed.formats.len() as u32;
                (*existing).p_view_formats = boxed.formats.as_ptr();
            }
        } else {
            boxed.list.view_format_count = boxed.formats.len() as u32;
            boxed.list.p_view_formats = boxed.formats.as_ptr();
            boxed.list.p_next = host.p_next;
            boxed.info.p_next = ptr::from_ref(&boxed.list).cast();
        }
        return Some(boxed);
    }
    list.view_format_count = 0;
    // SAFETY: the lifetime is erased; the box owns what the info needs.
    let usage = info.image_usage;
    Some(Box::new(Modified {
        info: unsafe {
            std::mem::transmute::<vk::SwapchainCreateInfoKHR<'_>, vk::SwapchainCreateInfoKHR<'static>>(
                info,
            )
        },
        formats,
        list,
        usage,
    }))
}

/// # Safety
/// `head` must start a valid `pNext` chain.
unsafe fn find_format_list(
    head: *const std::ffi::c_void,
) -> Option<*mut vk::ImageFormatListCreateInfo<'static>> {
    let mut node = head.cast::<vk::BaseOutStructure<'static>>();
    while !node.is_null() {
        // SAFETY: every node of a pNext chain starts with sType and pNext.
        unsafe {
            if (*node).s_type == vk::StructureType::IMAGE_FORMAT_LIST_CREATE_INFO {
                return Some(node.cast_mut().cast());
            }
            node = (*node).p_next;
        }
    }
    None
}

pub(super) unsafe extern "system" fn create_swapchain(
    device: vk::Device,
    p_create_info: *const vk::SwapchainCreateInfoKHR<'_>,
    p_allocator: *const vk::AllocationCallbacks<'_>,
    p_swapchain: *mut vk::SwapchainKHR,
) -> vk::Result {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded(
        "create_swapchain",
        vk::Result::ERROR_INITIALIZATION_FAILED,
        || unsafe { create_swapchain_impl(device, p_create_info, p_allocator, p_swapchain) },
    )
}

unsafe fn create_swapchain_impl(
    device: vk::Device,
    p_create_info: *const vk::SwapchainCreateInfoKHR<'_>,
    p_allocator: *const vk::AllocationCallbacks<'_>,
    p_swapchain: *mut vk::SwapchainKHR,
) -> vk::Result {
    let Some(data) = state::with(|registry| registry.devices.get(&device.as_raw()).cloned()) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    let create = data.swapchain.create_swapchain_khr;
    // SAFETY: the host's create info is valid for the call.
    let modified = guarded("swapchain setup", None, || unsafe {
        modify(&data, &*p_create_info)
    });
    let mut result = vk::Result::ERROR_INITIALIZATION_FAILED;
    let mut ours = false;
    if let Some(modified) = &modified {
        // SAFETY: valid create info; the host's allocator and out pointer.
        result = unsafe { create(device, &modified.info, p_allocator, p_swapchain) };
        ours = result == vk::Result::SUCCESS;
        if !ours {
            log::warn!("z-hook: the swapchain did not accept the overlay's additions ({result:?}); retrying unchanged");
        }
    }
    if !ours {
        // SAFETY: the host's own create info.
        result = unsafe { create(device, p_create_info, p_allocator, p_swapchain) };
    }
    if result == vk::Result::SUCCESS && ours {
        // SAFETY: creation succeeded.
        let handle = unsafe { *p_swapchain };
        // SAFETY: the host's create info is valid for the call.
        let host = unsafe { &*p_create_info };
        if let Some(modified) = &modified {
            guarded("swapchain registration", (), || {
                register(&data, handle, host, modified)
            });
        }
    }
    result
}

fn register(
    device: &Arc<DeviceData>,
    handle: vk::SwapchainKHR,
    host: &vk::SwapchainCreateInfoKHR<'_>,
    modified: &Modified,
) {
    let Some(images) = images_of(device, handle) else {
        log::warn!("z-hook: cannot list the swapchain images");
        return;
    };
    let semaphores: Result<Vec<_>, _> = images
        .iter()
        // SAFETY: plain object creation on a live device.
        .map(|_| unsafe {
            device
                .raw
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
        })
        .collect();
    let Ok(semaphores) = semaphores else {
        log::warn!("z-hook: cannot create the overlay's semaphores");
        return;
    };
    let data = Arc::new(SwapchainData {
        handle,
        surface: host.surface,
        device: Arc::clone(device),
        format: host.image_format,
        extent: host.image_extent,
        usage: modified.usage,
        images,
        semaphores: Mutex::new(semaphores),
    });
    state::with(|registry| registry.swapchains.insert(handle.as_raw(), data));
}

fn images_of(device: &DeviceData, swapchain: vk::SwapchainKHR) -> Option<Vec<vk::Image>> {
    let get = device.swapchain.get_swapchain_images_khr;
    let mut count = 0u32;
    // SAFETY: the standard two-call enumeration of a swapchain just created on this device.
    unsafe {
        if get(device.handle, swapchain, &mut count, ptr::null_mut()) != vk::Result::SUCCESS {
            return None;
        }
        let mut images = vec![vk::Image::null(); count as usize];
        if get(device.handle, swapchain, &mut count, images.as_mut_ptr()) != vk::Result::SUCCESS {
            return None;
        }
        images.truncate(count as usize);
        Some(images)
    }
}

pub(super) unsafe extern "system" fn destroy_swapchain(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    p_allocator: *const vk::AllocationCallbacks<'_>,
) {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded("destroy_swapchain", (), || unsafe {
        destroy_swapchain_impl(device, swapchain, p_allocator)
    })
}

unsafe fn destroy_swapchain_impl(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    p_allocator: *const vk::AllocationCallbacks<'_>,
) {
    let Some(data) = state::with(|registry| registry.devices.get(&device.as_raw()).cloned()) else {
        return;
    };
    let (ours, input) = state::with(|registry| {
        if registry.target == Some(swapchain.as_raw()) {
            registry.target = None;
        }
        #[cfg(all(feature = "x11", unix, not(target_os = "macos")))]
        let input = registry.inputs.remove(&swapchain.as_raw());
        #[cfg(not(all(feature = "x11", unix, not(target_os = "macos"))))]
        let input = None::<()>;
        (registry.swapchains.remove(&swapchain.as_raw()), input)
    });
    // Stopping the reader waits for its thread: outside the registry's lock.
    let _ = input;
    if let Some(ours) = ours {
        // The overlay's work on these images must be done before the images go away.
        data.wait_gpu();
        for semaphore in ours
            .semaphores
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drain(..)
        {
            // SAFETY: the host waits for the swapchain to be idle before destroying it, and
            // the present that waited on the semaphore is part of that.
            unsafe { data.raw.destroy_semaphore(semaphore, None) };
        }
    }
    let destroy = data.swapchain.destroy_swapchain_khr;
    // SAFETY: the next layer's function with the host's arguments.
    unsafe { destroy(device, swapchain, p_allocator) };
}
