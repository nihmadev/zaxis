//! `vkCreateInstance` and `vkDestroyInstance`.

use super::{
    ffi::{self, guarded, LayerInstanceLink},
    state::{self, InstanceData},
};
use ash::vk::{self, Handle};
use std::{
    collections::HashSet,
    ffi::{c_char, CStr, CString},
    sync::{Arc, Mutex},
};

const GET_PHYSICAL_DEVICE_PROPERTIES2: &CStr = c"VK_KHR_get_physical_device_properties2";

/// The host's create info with what the overlay needs on top: Vulkan 1.3, the version wgpu
/// creates its own instances with (it uses the core functions up to it; the driver exposes them
/// only to an instance created with a version that includes them) and `VK_KHR_get_physical_device_properties2` (it reads features through it).
///
/// Nothing is asked of the layers below first: enumerating instance extensions through the
/// chain with a null instance is not supported by every layer (Mesa's device-select layer
/// crashes on it). A driver that cannot give what is added fails the creation, and the caller
/// then creates the host's instance unchanged.
struct Modified {
    app: vk::ApplicationInfo<'static>,
    names: Vec<*const c_char>,
    _owned: Vec<CString>,
    info: vk::InstanceCreateInfo<'static>,
    version: u32,
}

/// # Safety
/// `original` must be the create info the loader passed.
unsafe fn modify(original: &vk::InstanceCreateInfo<'_>) -> Box<Modified> {
    // SAFETY: the host's create info is valid for the call.
    let app = unsafe { original.p_application_info.as_ref() }
        .copied()
        .unwrap_or_default();
    // SAFETY: the lifetime is erased; only plain data and strings the host keeps alive.
    let mut app: vk::ApplicationInfo<'static> = unsafe { std::mem::transmute(app) };
    let version = app.api_version.max(vk::API_VERSION_1_3);
    app.api_version = version;
    app.p_next = std::ptr::null();
    // SAFETY: the create info's name array is valid for its count.
    let host_names = unsafe { names(original) };
    let mut owned = Vec::new();
    let mut names: Vec<*const c_char> = host_names.iter().map(|name| name.as_ptr()).collect();
    if !host_names.contains(&GET_PHYSICAL_DEVICE_PROPERTIES2) {
        owned.push(GET_PHYSICAL_DEVICE_PROPERTIES2.to_owned());
        names.push(owned[0].as_ptr());
    }
    // SAFETY: the lifetime is erased; `Modified` keeps `app` and `names` alive until the call.
    let mut info: vk::InstanceCreateInfo<'static> = unsafe { std::mem::transmute(*original) };
    info.enabled_extension_count = names.len() as u32;
    // The pointers are set once the value is on the heap, where it stays.
    let mut modified = Box::new(Modified {
        app,
        names,
        _owned: owned,
        info,
        version,
    });
    modified.info.pp_enabled_extension_names = modified.names.as_ptr();
    modified.info.p_application_info = &modified.app;
    modified
}

/// # Safety
/// The extension names of `info` must be valid C strings.
pub(super) unsafe fn names<'a>(info: &vk::InstanceCreateInfo<'_>) -> Vec<&'a CStr> {
    if info.pp_enabled_extension_names.is_null() {
        return Vec::new();
    }
    (0..info.enabled_extension_count as usize)
        // SAFETY: the array has `enabled_extension_count` valid C strings.
        .map(|i| unsafe { CStr::from_ptr(*info.pp_enabled_extension_names.add(i)) })
        .collect()
}

pub(super) unsafe extern "system" fn create_instance(
    p_create_info: *const vk::InstanceCreateInfo<'_>,
    p_allocator: *const vk::AllocationCallbacks<'_>,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded(
        "create_instance",
        vk::Result::ERROR_INITIALIZATION_FAILED,
        || unsafe { create_instance_impl(p_create_info, p_allocator, p_instance) },
    )
}

unsafe fn create_instance_impl(
    p_create_info: *const vk::InstanceCreateInfo<'_>,
    p_allocator: *const vk::AllocationCallbacks<'_>,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    // SAFETY: the loader passes a valid create info whose pNext chain carries its link.
    let Some(link) = (unsafe {
        ffi::take_link::<LayerInstanceLink>(
            (*p_create_info).p_next,
            ffi::LOADER_INSTANCE_CREATE_INFO,
        )
    }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: the link is the loader's.
    let next_gipa = unsafe { (*link).next_gipa };
    // SAFETY: a null instance asks for the global command.
    let Some(create) = (unsafe { next_gipa(vk::Instance::null(), c"vkCreateInstance".as_ptr()) })
    else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    // SAFETY: the next layer returns the real entry point for this name.
    let create: vk::PFN_vkCreateInstance = unsafe { std::mem::transmute(create) };
    let overlay = crate::registry::current().is_some();
    let modified = if overlay {
        // SAFETY: see `modify`.
        guarded("instance setup", None, || {
            Some(unsafe { modify(&*p_create_info) })
        })
    } else {
        None
    };
    let mut result = vk::Result::ERROR_INITIALIZATION_FAILED;
    let mut version = 0;
    if let Some(modified) = &modified {
        // SAFETY: valid create info, the host's allocator and out pointer.
        result = unsafe { create(&modified.info, p_allocator, p_instance) };
        version = modified.version;
        if result != vk::Result::SUCCESS {
            log::warn!("z-hook: the instance did not accept the overlay's additions ({result:?}); retrying unchanged");
        }
    }
    if result != vk::Result::SUCCESS {
        // SAFETY: the host's own create info.
        result = unsafe { create(p_create_info, p_allocator, p_instance) };
        // SAFETY: the host's own create info.
        version = unsafe { (*p_create_info).p_application_info.as_ref() }
            .map_or(vk::API_VERSION_1_0, |a| a.api_version);
    }
    if result != vk::Result::SUCCESS {
        return result;
    }
    // SAFETY: creation succeeded, so the out pointer holds the instance.
    let handle = unsafe { *p_instance };
    // SAFETY: the host's own create info is valid for the call.
    let enabled = unsafe { names(&modified.as_ref().map_or(*p_create_info, |m| m.info)) };
    let extensions: HashSet<String> = enabled
        .iter()
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    let static_fn = ash::StaticFn {
        get_instance_proc_addr: next_gipa,
    };
    // SAFETY: `handle` was created through this chain.
    let raw = unsafe { ash::Instance::load(&static_fn, handle) };
    // The entry's global functions are left unresolved: the layers below do not all answer
    // for a null instance. wgpu only uses it to load surface functions for a real instance.
    let unresolved = ash::EntryFnV1_0::load(|_| std::ptr::null());
    let unresolved_1_1 = ash::EntryFnV1_1::load(|_| std::ptr::null());
    let entry = ash::Entry::from_parts_1_1(static_fn, unresolved, unresolved_1_1);
    let data = Arc::new(InstanceData {
        handle,
        next_gipa,
        raw,
        entry,
        api_version: version,
        extensions,
        wgpu: Mutex::new(None),
    });
    drop(modified);
    state::with(|registry| registry.instances.insert(handle.as_raw(), data));
    vk::Result::SUCCESS
}

pub(super) unsafe extern "system" fn destroy_instance(
    instance: vk::Instance,
    p_allocator: *const vk::AllocationCallbacks<'_>,
) {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded("destroy_instance", (), || unsafe {
        destroy_instance_impl(instance, p_allocator)
    })
}

unsafe fn destroy_instance_impl(
    instance: vk::Instance,
    p_allocator: *const vk::AllocationCallbacks<'_>,
) {
    let data = state::with(|registry| registry.instances.remove(&instance.as_raw()));
    let Some(data) = data else { return };
    drop(
        data.wgpu
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take(),
    );
    let destroy = data.raw.fp_v1_0().destroy_instance;
    // SAFETY: the next layer's function with the host's arguments.
    unsafe { destroy(instance, p_allocator) };
}
