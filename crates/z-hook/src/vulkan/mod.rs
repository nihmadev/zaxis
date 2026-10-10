//! The Vulkan implicit layer: draws into the swapchain images of the host through
//! `vkQueuePresentKHR`.
//!
//! The Vulkan loader loads the layer's library and asks it for entry points; the library is
//! yours (a `cdylib` that depends on this crate) and exports one symbol, created by
//! [`vulkan_layer!`](crate::vulkan_layer), which negotiates the interface and installs the
//! overlay the first time. The layer then wraps instance, device, swapchain and present:
//!
//! * `vkCreateInstance` raises the API version to 1.1 for a host that asked for 1.0 and enables
//!   `VK_KHR_get_physical_device_properties2`, so wgpu can inspect the device;
//! * `vkCreateDevice` adds the extensions and features wgpu would have requested to the host's
//!   own, and falls back to the host's untouched request when the driver refuses;
//! * `vkCreateSwapchainKHR` adds `COLOR_ATTACHMENT` and `TRANSFER_SRC` usage and, for `UNORM`
//!   formats, the sRGB sibling as a view format, so the interface blends in sRGB;
//! * `vkQueuePresentKHR` renders the interface into the image about to be presented, on the
//!   host's own queue, waits for the host's semaphores and replaces them with its own.

mod backend;
mod device;
mod dump;
mod features;
mod ffi;
mod gpu;
mod input;
mod instance;
mod layer;
mod present;
mod state;
mod surface;
mod swapchain;
#[cfg(all(feature = "x11", unix, not(target_os = "macos")))]
mod x11;

pub use ffi::NegotiateLayerInterface;
pub use layer::{negotiate_layer, LayerEntry};

/// Export the layer's negotiation symbol from your `cdylib`, with the overlay it installs.
///
/// ```ignore
/// z_hook::vulkan_layer!(|| (
///     z_hook::OverlayOptions::default(),
///     |ctx: &mut zaxis::Context| { /* build the interface */ },
/// ));
/// ```
///
/// The closure runs once, when the Vulkan loader first negotiates with the layer, on the
/// loader's thread (never inside `DllMain`). Describe the layer in a JSON manifest (see the
/// documentation) and point the loader at it.
#[macro_export]
macro_rules! vulkan_layer {
    ($factory:expr) => {
        /// Entry point the Vulkan loader looks up in a layer's library.
        #[allow(unsafe_code, non_snake_case, clippy::missing_safety_doc)]
        #[no_mangle]
        pub unsafe extern "system" fn vkNegotiateLoaderLayerInterfaceVersion(
            interface: *mut $crate::vulkan::NegotiateLayerInterface,
        ) -> i32 {
            // SAFETY: the loader passes its negotiation structure.
            unsafe { $crate::vulkan::negotiate_layer(interface, $factory) }
        }
    };
}
