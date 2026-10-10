//! Surfaces: which window of the host a swapchain presents to, to find its input.

use super::{
    ffi::guarded,
    state::{self, InstanceData},
};
use ash::vk::{self, Handle};
use std::sync::Arc;

/// The native window behind a surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HostWindow {
    /// An X11 window (XID), through Xlib or XCB.
    X11(u32),
    /// A Wayland surface: the layer has no access to the host's events there.
    Wayland,
}

fn instance_of(handle: vk::Instance) -> Option<Arc<InstanceData>> {
    state::with(|registry| registry.instances.get(&handle.as_raw()).cloned())
}

fn remember(surface: vk::SurfaceKHR, window: HostWindow) {
    state::with(|registry| registry.surfaces.insert(surface.as_raw(), window));
}

macro_rules! surface_hook {
    ($name:ident, $impl_name:ident, $pfn:ty, $info:ty, $c_name:literal, |$i:ident| $window:expr) => {
        pub(super) unsafe extern "system" fn $name(
            instance: vk::Instance,
            p_create_info: *const $info,
            p_allocator: *const vk::AllocationCallbacks<'_>,
            p_surface: *mut vk::SurfaceKHR,
        ) -> vk::Result {
            // SAFETY: the arguments are the host's, passed on unchanged.
            guarded(
                stringify!($name),
                vk::Result::ERROR_INITIALIZATION_FAILED,
                || unsafe { $impl_name(instance, p_create_info, p_allocator, p_surface) },
            )
        }

        unsafe fn $impl_name(
            instance: vk::Instance,
            p_create_info: *const $info,
            p_allocator: *const vk::AllocationCallbacks<'_>,
            p_surface: *mut vk::SurfaceKHR,
        ) -> vk::Result {
            let Some(data) = instance_of(instance) else {
                return vk::Result::ERROR_INITIALIZATION_FAILED;
            };
            // SAFETY: the next layer resolves the name for this instance.
            let Some(next) = (unsafe { (data.next_gipa)(instance, $c_name.as_ptr()) }) else {
                return vk::Result::ERROR_EXTENSION_NOT_PRESENT;
            };
            // SAFETY: the name says what the pointer is.
            let next: $pfn = unsafe { std::mem::transmute(next) };
            // SAFETY: the next layer's function with the host's arguments.
            let result = unsafe { next(instance, p_create_info, p_allocator, p_surface) };
            if result == vk::Result::SUCCESS {
                // SAFETY: the host's create info is valid, and the surface was written.
                let ($i, surface) = unsafe { (&*p_create_info, *p_surface) };
                remember(surface, $window);
            }
            result
        }
    };
}

surface_hook!(
    create_xcb_surface,
    create_xcb_surface_impl,
    vk::PFN_vkCreateXcbSurfaceKHR,
    vk::XcbSurfaceCreateInfoKHR<'_>,
    c"vkCreateXcbSurfaceKHR",
    |info| HostWindow::X11(info.window)
);
surface_hook!(
    create_xlib_surface,
    create_xlib_surface_impl,
    vk::PFN_vkCreateXlibSurfaceKHR,
    vk::XlibSurfaceCreateInfoKHR<'_>,
    c"vkCreateXlibSurfaceKHR",
    |info| HostWindow::X11(info.window as _)
);
surface_hook!(
    create_wayland_surface,
    create_wayland_surface_impl,
    vk::PFN_vkCreateWaylandSurfaceKHR,
    vk::WaylandSurfaceCreateInfoKHR<'_>,
    c"vkCreateWaylandSurfaceKHR",
    |info| {
        let _ = info;
        HostWindow::Wayland
    }
);

pub(super) unsafe extern "system" fn destroy_surface(
    instance: vk::Instance,
    surface: vk::SurfaceKHR,
    p_allocator: *const vk::AllocationCallbacks<'_>,
) {
    let Some(data) = instance_of(instance) else {
        return;
    };
    state::with(|registry| registry.surfaces.remove(&surface.as_raw()));
    // SAFETY: the next layer resolves the name for this instance.
    let Some(next) = (unsafe { (data.next_gipa)(instance, c"vkDestroySurfaceKHR".as_ptr()) })
    else {
        return;
    };
    // SAFETY: the name says what the pointer is.
    let next: vk::PFN_vkDestroySurfaceKHR = unsafe { std::mem::transmute(next) };
    // SAFETY: the next layer's function with the host's arguments.
    unsafe { next(instance, surface, p_allocator) };
}
