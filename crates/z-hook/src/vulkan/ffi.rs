//! The loader/layer interface of `vk_layer.h`, which `ash` does not carry, and the plumbing
//! every intercepted entry point shares.

use ash::vk;
use std::{
    ffi::{c_char, c_void, CStr},
    panic::{catch_unwind, AssertUnwindSafe},
};

pub(super) type Gipa = vk::PFN_vkGetInstanceProcAddr;
pub(super) type Gdpa = vk::PFN_vkGetDeviceProcAddr;
pub(super) type VoidFn = vk::PFN_vkVoidFunction;

/// `VK_STRUCTURE_TYPE_LOADER_INSTANCE_CREATE_INFO` and `..._DEVICE_CREATE_INFO`.
pub(super) const LOADER_INSTANCE_CREATE_INFO: i32 = 47;
pub(super) const LOADER_DEVICE_CREATE_INFO: i32 = 48;
/// `VK_LAYER_LINK_INFO`: the chain link in a `VkLayer*CreateInfo`.
pub(super) const LAYER_LINK_INFO: u32 = 0;
/// `VK_LOADER_DATA_CALLBACK`: the loader's function that gives a dispatchable object its
/// dispatch table pointer.
const LOADER_DATA_CALLBACK: u32 = 1;

/// `PFN_vkSetDeviceLoaderData`: every dispatchable object a layer creates itself (a command
/// buffer) must pass through it, or the layers below cannot find their data for it.
pub(super) type SetDeviceLoaderData =
    unsafe extern "system" fn(vk::Device, *mut c_void) -> vk::Result;
/// `LAYER_NEGOTIATE_INTERFACE_STRUCT`.
const NEGOTIATE_INTERFACE_STRUCT: u32 = 1;
/// The loader/layer interface version this layer implements: GIPA/GDPA come from the
/// negotiation, not from exported symbols.
const INTERFACE_VERSION: u32 = 2;

#[repr(C)]
pub struct NegotiateLayerInterface {
    s_type: u32,
    p_next: *mut c_void,
    loader_layer_interface_version: u32,
    pfn_get_instance_proc_addr: Option<Gipa_>,
    pfn_get_device_proc_addr: Option<Gdpa_>,
    pfn_get_physical_device_proc_addr:
        Option<unsafe extern "system" fn(vk::Instance, *const c_char) -> VoidFn>,
}

type Gipa_ = unsafe extern "system" fn(vk::Instance, *const c_char) -> VoidFn;
type Gdpa_ = unsafe extern "system" fn(vk::Device, *const c_char) -> VoidFn;

#[repr(C)]
pub(super) struct LayerInstanceLink {
    pub p_next: *mut LayerInstanceLink,
    pub next_gipa: Gipa,
    pub next_gpdpa: Option<unsafe extern "system" fn(vk::Instance, *const c_char) -> VoidFn>,
}

#[repr(C)]
pub(super) struct LayerDeviceLink {
    pub p_next: *mut LayerDeviceLink,
    pub next_gipa: Gipa,
    pub next_gdpa: Gdpa,
}

#[repr(C)]
pub(super) struct LayerCreateInfo<L> {
    pub s_type: i32,
    pub p_next: *const c_void,
    pub function: u32,
    pub link: *mut L,
}

/// Find the loader's chain link in the `pNext` chain of a create info and advance it, so the
/// next layer finds its own. `s_type` selects instance or device chains.
///
/// # Safety
/// `chain` must be the `pNext` of a create info the loader passed to a layer.
pub(super) unsafe fn take_link<L>(chain: *const c_void, s_type: i32) -> Option<*mut L>
where
    L: HasNext,
{
    let mut node = chain.cast::<LayerCreateInfo<L>>();
    while !node.is_null() {
        // SAFETY: every node of a Vulkan pNext chain starts with sType and pNext.
        let (kind, next) = unsafe { ((*node).s_type, (*node).p_next) };
        if kind == s_type {
            // SAFETY: the sType says this node is a VkLayer*CreateInfo.
            let info = unsafe { &mut *node.cast_mut() };
            if info.function == LAYER_LINK_INFO && !info.link.is_null() {
                let link = info.link;
                // SAFETY: the loader built a valid, null-terminated list of links.
                info.link = unsafe { (*link).next() };
                return Some(link);
            }
        }
        node = next.cast();
    }
    None
}

pub(super) trait HasNext {
    fn next(&self) -> *mut Self;
}

impl HasNext for LayerInstanceLink {
    fn next(&self) -> *mut Self {
        self.p_next
    }
}

impl HasNext for LayerDeviceLink {
    fn next(&self) -> *mut Self {
        self.p_next
    }
}

/// Run an intercepted entry point so that nothing unwinds into the host: a panic is logged and
/// `fallback` is returned (callers that must still forward to the next layer do so outside).
pub(super) fn guarded<R>(what: &str, fallback: R, body: impl FnOnce() -> R) -> R {
    match catch_unwind(AssertUnwindSafe(body)) {
        Ok(value) => value,
        Err(payload) => {
            log::error!(
                "z-hook: panic in {what}: {}",
                crate::driver::panic_message(&payload)
            );
            fallback
        }
    }
}

/// `name` as a `&str` for matching, or `""`.
///
/// # Safety
/// `name` must be null or a valid C string.
pub(super) unsafe fn name_of<'a>(name: *const c_char) -> &'a str {
    if name.is_null() {
        return "";
    }
    // SAFETY: the caller promises a valid C string.
    unsafe { CStr::from_ptr(name) }.to_str().unwrap_or("")
}

/// Fill the loader's negotiation structure with this layer's entry points.
///
/// # Safety
/// `interface` must be null or point to the loader's `VkNegotiateLayerInterface`.
pub(super) unsafe fn negotiate(
    interface: *mut NegotiateLayerInterface,
    gipa: Gipa_,
    gdpa: Gdpa_,
) -> vk::Result {
    // SAFETY: checked for null; the loader passes a valid structure.
    let Some(interface) = (unsafe { interface.as_mut() }) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    if interface.s_type != NEGOTIATE_INTERFACE_STRUCT {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    if interface.loader_layer_interface_version < INTERFACE_VERSION {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }
    interface.loader_layer_interface_version = INTERFACE_VERSION;
    interface.pfn_get_instance_proc_addr = Some(gipa);
    interface.pfn_get_device_proc_addr = Some(gdpa);
    interface.pfn_get_physical_device_proc_addr = None;
    vk::Result::SUCCESS
}

/// The loader's `vkSetDeviceLoaderData` callback in a device create info's chain.
///
/// # Safety
/// `chain` must be the `pNext` of a create info the loader passed to a layer.
pub(super) unsafe fn find_loader_data(chain: *const c_void) -> Option<SetDeviceLoaderData> {
    let mut node = chain.cast::<LayerCreateInfo<c_void>>();
    while !node.is_null() {
        // SAFETY: every node of a Vulkan pNext chain starts with sType and pNext.
        let (kind, next) = unsafe { ((*node).s_type, (*node).p_next) };
        if kind == LOADER_DEVICE_CREATE_INFO {
            // SAFETY: the sType says this node is a VkLayerDeviceCreateInfo.
            let info = unsafe { &*node };
            if info.function == LOADER_DATA_CALLBACK && !info.link.is_null() {
                // SAFETY: for this function code the union holds the callback, a pointer-sized
                // value in the place of `link`.
                return Some(unsafe {
                    std::mem::transmute::<*mut c_void, SetDeviceLoaderData>(info.link)
                });
            }
        }
        node = next.cast();
    }
    None
}
