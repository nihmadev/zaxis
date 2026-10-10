//! `vkCreateDevice`, `vkDestroyDevice` and the queue lookups.

use super::{
    features::{self, Merged},
    ffi::{self, guarded, Gdpa, LayerDeviceLink},
    gpu::GpuSlot,
    state::{self, DeviceData, InstanceData, Interop},
};
use ash::vk::{self, Handle};
use std::{
    collections::HashMap,
    ffi::CStr,
    sync::{Arc, Mutex, PoisonError},
};
use wgpu::hal::api::Vulkan;
use zaxis::wgpu;

const SWAPCHAIN: &CStr = c"VK_KHR_swapchain";

/// The wgpu instance over the host's Vulkan instance, made on first use.
pub(super) fn wgpu_instance(instance: &Arc<InstanceData>) -> Option<Arc<wgpu::Instance>> {
    let mut slot = instance.wgpu.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(existing) = slot.as_ref() {
        return Some(Arc::clone(existing));
    }
    let flags = wgpu::InstanceFlags::empty();
    // wgpu is told about the extensions it may use that the host's instance has enabled
    // (it assumes whatever it lists is enabled). Surfaces are never made through it.
    let known: [&'static CStr; 7] = [
        ash::khr::surface::NAME,
        ash::khr::xlib_surface::NAME,
        ash::khr::xcb_surface::NAME,
        ash::khr::wayland_surface::NAME,
        ash::khr::win32_surface::NAME,
        ash::ext::swapchain_colorspace::NAME,
        ash::khr::get_physical_device_properties2::NAME,
    ];
    let enabled: Vec<&'static CStr> = known
        .into_iter()
        .filter(|name| {
            instance
                .extensions
                .contains(name.to_string_lossy().as_ref())
        })
        .collect();
    // SAFETY: the raw instance was created through this layer chain with the version and
    // extensions passed here; a drop callback keeps wgpu from destroying the host's instance.
    let hal = unsafe {
        wgpu::hal::vulkan::Instance::from_raw(
            instance.entry.clone(),
            instance.raw.clone(),
            instance.api_version,
            0,
            None,
            enabled,
            flags,
            wgpu::MemoryBudgetThresholds::default(),
            false,
            Some(Box::new(|| {})),
        )
    }
    .map_err(|error| log::warn!("z-hook: wgpu cannot use the host's Vulkan instance: {error}"))
    .ok()?;
    // SAFETY: `hal` is a valid, usable Vulkan instance.
    let created = Arc::new(unsafe { wgpu::Instance::from_hal::<Vulkan>(hal) });
    *slot = Some(Arc::clone(&created));
    Some(created)
}

struct Plan {
    merged: Merged,
    interop: Interop,
}

/// What the host's `VkDeviceCreateInfo` needs for wgpu, or why it cannot get it.
///
/// # Safety
/// `host` must be the create info the host passed for `physical`.
unsafe fn plan(
    instance: &Arc<InstanceData>,
    physical: vk::PhysicalDevice,
    host: &vk::DeviceCreateInfo<'_>,
) -> Option<Plan> {
    let wgpu_instance = wgpu_instance(instance)?;
    // SAFETY: the instance is a Vulkan one.
    let hal = unsafe { wgpu_instance.as_hal::<Vulkan>() }?;
    let exposed = hal.expose_adapter(physical)?;
    let adapter = &exposed.adapter;
    let features = wgpu::Features::empty();
    let mut extensions = adapter.required_device_extensions(features);
    let supported =
        unsafe { instance.raw.enumerate_device_extension_properties(physical) }.unwrap_or_default();
    let has = |name: &CStr| {
        supported
            .iter()
            .any(|p| p.extension_name_as_c_str() == Ok(name))
    };
    let mut mutable_format = false;
    if has(ash::khr::swapchain_mutable_format::NAME) && has(ash::khr::image_format_list::NAME) {
        let before = extensions.len();
        extensions.push(ash::khr::swapchain_mutable_format::NAME);
        extensions.push(ash::khr::image_format_list::NAME);
        extensions.push(ash::khr::maintenance2::NAME);
        mutable_format = extensions.len() > before;
    }
    extensions.dedup();
    let mut wanted_features = adapter.physical_device_features(&extensions, features);
    let wanted = wanted_features.add_to_device_create(vk::DeviceCreateInfo::default());
    // SAFETY: `host` and `wanted` are valid create infos.
    let merged = unsafe { features::merge(host, &wanted, &extensions) }
        .map_err(|error| log::warn!("z-hook: {}", error.0))
        .ok()?;
    // SAFETY: plain query on a physical device of this instance.
    let families = unsafe {
        instance
            .raw
            .get_physical_device_queue_family_properties(physical)
    };
    let family_flags = families.iter().map(|f| f.queue_flags).collect();
    Some(Plan {
        merged,
        interop: Interop {
            mutable_format,
            extensions,
            family_flags,
        },
    })
}

fn owning_instance(physical: vk::PhysicalDevice) -> Option<Arc<InstanceData>> {
    let instances: Vec<_> = state::with(|registry| registry.instances.values().cloned().collect());
    instances.into_iter().find(|instance| {
        // SAFETY: plain enumeration on a live instance.
        unsafe { instance.raw.enumerate_physical_devices() }
            .is_ok_and(|all| all.contains(&physical))
    })
}

pub(super) unsafe extern "system" fn create_device(
    physical: vk::PhysicalDevice,
    p_create_info: *const vk::DeviceCreateInfo<'_>,
    p_allocator: *const vk::AllocationCallbacks<'_>,
    p_device: *mut vk::Device,
) -> vk::Result {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded(
        "create_device",
        vk::Result::ERROR_INITIALIZATION_FAILED,
        || unsafe { create_device_impl(physical, p_create_info, p_allocator, p_device) },
    )
}

unsafe fn create_device_impl(
    physical: vk::PhysicalDevice,
    p_create_info: *const vk::DeviceCreateInfo<'_>,
    p_allocator: *const vk::AllocationCallbacks<'_>,
    p_device: *mut vk::Device,
) -> vk::Result {
    // SAFETY: the loader passes a valid create info whose pNext chain carries its link.
    let Some(link) = (unsafe {
        ffi::take_link::<LayerDeviceLink>((*p_create_info).p_next, ffi::LOADER_DEVICE_CREATE_INFO)
    }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: the link is the loader's.
    let (next_gipa, next_gdpa) = unsafe { ((*link).next_gipa, (*link).next_gdpa) };
    // SAFETY: the same chain; found before the link was taken from it.
    let set_loader_data = unsafe { ffi::find_loader_data((*p_create_info).p_next) };
    let instance = owning_instance(physical);
    let instance_handle = instance.as_ref().map_or(vk::Instance::null(), |i| i.handle);
    // SAFETY: the next layer returns the real entry point for this name.
    let Some(create) = (unsafe { next_gipa(instance_handle, c"vkCreateDevice".as_ptr()) }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: as above.
    let create: vk::PFN_vkCreateDevice = unsafe { std::mem::transmute(create) };
    let planned = match (&instance, crate::registry::current()) {
        // SAFETY: the host's create info is valid for this call.
        (Some(instance), Some(_)) => guarded("device setup", None, || unsafe {
            plan(instance, physical, &*p_create_info)
        }),
        _ => None,
    };
    let mut interop = None;
    let mut result = vk::Result::ERROR_INITIALIZATION_FAILED;
    if let Some(planned) = planned {
        // SAFETY: a valid create info and the host's own allocator and out pointer.
        result = unsafe { create(physical, planned.merged.info(), p_allocator, p_device) };
        if result == vk::Result::SUCCESS {
            interop = Some(planned.interop);
        } else {
            log::warn!("z-hook: the device did not accept the overlay's additions ({result:?}); retrying unchanged");
        }
    }
    if result != vk::Result::SUCCESS {
        // SAFETY: the host's own create info.
        result = unsafe { create(physical, p_create_info, p_allocator, p_device) };
    }
    if result != vk::Result::SUCCESS {
        return result;
    }
    // SAFETY: creation succeeded.
    let handle = unsafe { *p_device };
    // SAFETY: the host's names are valid C strings.
    let uses_swapchain = unsafe { names_of(&*p_create_info) }.contains(&SWAPCHAIN);
    let (Some(instance), true) = (instance, uses_swapchain) else {
        return vk::Result::SUCCESS;
    };
    register(
        &instance,
        physical,
        handle,
        next_gdpa,
        set_loader_data,
        interop,
    );
    vk::Result::SUCCESS
}

/// # Safety
/// The extension names of `info` must be valid C strings.
unsafe fn names_of<'a>(info: &vk::DeviceCreateInfo<'_>) -> Vec<&'a CStr> {
    // SAFETY: the instance and device create infos have the same name-array layout rules.
    (0..info.enabled_extension_count as usize)
        .map(|i| unsafe { CStr::from_ptr(*info.pp_enabled_extension_names.add(i)) })
        .collect()
}

fn register(
    instance: &Arc<InstanceData>,
    physical: vk::PhysicalDevice,
    handle: vk::Device,
    next_gdpa: Gdpa,
    set_loader_data: Option<ffi::SetDeviceLoaderData>,
    interop: Option<Interop>,
) {
    let mut load = |name: &CStr| -> *const std::ffi::c_void {
        // SAFETY: the next layer resolves the name for this device.
        unsafe { next_gdpa(handle, name.as_ptr()) }.map_or(std::ptr::null(), |function| {
            function as *const std::ffi::c_void
        })
    };
    let mut fn_1_0 = ash::DeviceFnV1_0::load(&mut load);
    let next_allocate_command_buffers = fn_1_0.allocate_command_buffers;
    // wgpu allocates command buffers itself, below the loader; they need the loader's dispatch
    // pointer, which only `set_loader_data` writes.
    if set_loader_data.is_some() {
        fn_1_0.allocate_command_buffers = allocate_command_buffers;
    }
    let raw = ash::Device::from_parts_1_3(
        handle,
        fn_1_0,
        ash::DeviceFnV1_1::load(&mut load),
        ash::DeviceFnV1_2::load(&mut load),
        ash::DeviceFnV1_3::load(&mut load),
    );
    let swapchain = ash::khr::swapchain::DeviceFn::load(&mut load);
    let data = Arc::new(DeviceData {
        handle,
        physical,
        instance: Arc::clone(instance),
        raw,
        swapchain,
        next_gdpa,
        set_loader_data,
        next_allocate_command_buffers,
        interop: interop.filter(|_| set_loader_data.is_some()),
        gpu: Mutex::new(GpuSlot::Idle),
        worker: Mutex::new(None),
        closing: std::sync::atomic::AtomicBool::new(false),
        queues: Mutex::new(HashMap::new()),
    });
    state::with(|registry| registry.devices.insert(handle.as_raw(), data));
}

/// `vkAllocateCommandBuffers` as wgpu calls it: the next layer's, then the loader's dispatch
/// pointer into each buffer.
///
/// A dispatchable object starts with the loader's dispatch table pointer, which the loader
/// writes for the objects it creates. For the buffers wgpu creates below it, the layer must
/// write it: by `vkSetDeviceLoaderData`, or by copying the device's own, which is all that
/// callback does. The callback searches the loader's instance list under its global lock, which
/// the thread inside `vkDestroyDevice` holds while it waits for the initializing thread to
/// finish, so the copy is the one that cannot deadlock; the callback stays as the way when the
/// device's pointer is not set.
unsafe extern "system" fn allocate_command_buffers(
    device: vk::Device,
    p_info: *const vk::CommandBufferAllocateInfo<'_>,
    p_buffers: *mut vk::CommandBuffer,
) -> vk::Result {
    let Some(data) = state::with(|registry| registry.devices.get(&device.as_raw()).cloned()) else {
        return vk::Result::ERROR_DEVICE_LOST;
    };
    // SAFETY: the next layer's function with the caller's arguments.
    let result = unsafe { (data.next_allocate_command_buffers)(device, p_info, p_buffers) };
    if result != vk::Result::SUCCESS {
        return result;
    }
    // SAFETY: a dispatchable handle is a pointer to an object that starts with the dispatch pointer.
    let table = unsafe { *(device.as_raw() as *const *mut std::ffi::c_void) };
    // SAFETY: the allocation wrote `command_buffer_count` handles.
    unsafe {
        for i in 0..(*p_info).command_buffer_count as usize {
            let buffer = (*p_buffers.add(i)).as_raw() as *mut *mut std::ffi::c_void;
            if !table.is_null() {
                *buffer = table;
            } else if let Some(set) = data.set_loader_data {
                let status = set(device, buffer.cast());
                if status != vk::Result::SUCCESS {
                    return status;
                }
            }
        }
    }
    result
}

pub(super) unsafe extern "system" fn destroy_device(
    device: vk::Device,
    p_allocator: *const vk::AllocationCallbacks<'_>,
) {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded("destroy_device", (), || unsafe {
        destroy_device_impl(device, p_allocator)
    })
}

unsafe fn destroy_device_impl(device: vk::Device, p_allocator: *const vk::AllocationCallbacks<'_>) {
    let data = state::with(|registry| {
        registry.queues.retain(|_, owner| owner.handle != device);
        registry.devices.remove(&device.as_raw())
    });
    let Some(data) = data else { return };
    // Everything wgpu made on the device goes before the device does.
    data.release_gpu();
    let destroy = data.raw.fp_v1_0().destroy_device;
    // SAFETY: the next layer's function with the host's arguments.
    unsafe { destroy(device, p_allocator) };
}

impl DeviceData {
    fn remember_queue(self: &Arc<Self>, queue: vk::Queue, family: u32, index: u32) {
        if queue == vk::Queue::null() {
            return;
        }
        self.queues
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(queue.as_raw(), (family, index));
        state::with(|registry| registry.queues.insert(queue.as_raw(), Arc::clone(self)));
    }
}

pub(super) unsafe extern "system" fn get_device_queue(
    device: vk::Device,
    family: u32,
    index: u32,
    p_queue: *mut vk::Queue,
) {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded("get_device_queue", (), || unsafe {
        get_device_queue_impl(device, family, index, p_queue)
    })
}

unsafe fn get_device_queue_impl(
    device: vk::Device,
    family: u32,
    index: u32,
    p_queue: *mut vk::Queue,
) {
    let Some(data) = state::with(|registry| registry.devices.get(&device.as_raw()).cloned()) else {
        return;
    };
    let get = data.raw.fp_v1_0().get_device_queue;
    // SAFETY: the next layer's function with the host's arguments.
    unsafe { get(device, family, index, p_queue) };
    // SAFETY: the function wrote the queue.
    data.remember_queue(unsafe { *p_queue }, family, index);
}

pub(super) unsafe extern "system" fn get_device_queue2(
    device: vk::Device,
    p_info: *const vk::DeviceQueueInfo2<'_>,
    p_queue: *mut vk::Queue,
) {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded("get_device_queue2", (), || unsafe {
        get_device_queue2_impl(device, p_info, p_queue)
    })
}

unsafe fn get_device_queue2_impl(
    device: vk::Device,
    p_info: *const vk::DeviceQueueInfo2<'_>,
    p_queue: *mut vk::Queue,
) {
    let Some(data) = state::with(|registry| registry.devices.get(&device.as_raw()).cloned()) else {
        return;
    };
    // SAFETY: resolved through the chain; Vulkan 1.1 core.
    let Some(get) = (unsafe { (data.next_gdpa)(device, c"vkGetDeviceQueue2".as_ptr()) }) else {
        return;
    };
    // SAFETY: the name says what the pointer is.
    let get: vk::PFN_vkGetDeviceQueue2 = unsafe { std::mem::transmute(get) };
    // SAFETY: the next layer's function with the host's arguments.
    unsafe { get(device, p_info, p_queue) };
    // SAFETY: the host's info is valid, and the function wrote the queue.
    let (info, queue) = unsafe { (*p_info, *p_queue) };
    data.remember_queue(queue, info.queue_family_index, info.queue_index);
}
