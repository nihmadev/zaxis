//! Where the Vulkan layer gets the host's input from.
//!
//! * X11 (including XWayland): observed through XInput2 (see [`x11`](super::x11)).
//! * Wayland: nothing. The compositor sends a client's input events to that client's own
//!   objects; a layer inside the process cannot read them without taking the host's objects
//!   over, which the layer does not do. The overlay shows, and an application or mod that has
//!   the host's events feeds them through [`input_sink`](crate::input_sink).

use super::{
    state::{self, SwapchainData},
    surface::HostWindow,
};
use crate::core::Core;
use ash::vk::Handle;
use std::sync::{
    atomic::{AtomicBool, Ordering::Relaxed},
    Arc,
};

static WAYLAND_NOTICE: AtomicBool = AtomicBool::new(false);

/// Start reading the input of the window `swapchain` presents to, if the layer can.
pub(super) fn start(core: &Arc<Core>, swapchain: &Arc<SwapchainData>) {
    let window =
        state::with(|registry| registry.surfaces.get(&swapchain.surface.as_raw()).copied());
    match window {
        #[cfg(all(feature = "x11", unix, not(target_os = "macos")))]
        Some(HostWindow::X11(id)) => {
            if let Some(reader) = super::x11::X11Input::start(Arc::clone(core), id) {
                state::with(|registry| registry.inputs.insert(swapchain.handle.as_raw(), reader));
            }
        }
        #[cfg(not(all(feature = "x11", unix, not(target_os = "macos"))))]
        Some(HostWindow::X11(_)) => {
            log::info!("z-hook: built without the `x11` feature: the overlay gets no input")
        }
        Some(HostWindow::Wayland) => {
            if !WAYLAND_NOTICE.swap(true, Relaxed) {
                log::info!("z-hook: the host is a Wayland client; its input is not visible to a layer, so the overlay only shows (feed it through `input_sink()`)");
            }
        }
        None => log::info!("z-hook: unknown window system; the overlay only shows"),
    }
    let _ = core;
}
