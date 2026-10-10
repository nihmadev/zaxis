//! Windows: the overlay over Direct3D 11 and Direct3D 12 swapchains.
//!
//! `IDXGISwapChain::Present`, `Present1`, `ResizeBuffers` and `ResizeBuffers1` are hooked by
//! replacing their slots in the swapchain class's vtable (see [`vtable`]). On a present, the
//! overlay is drawn by wgpu on a Direct3D 12 device of its own, on the same adapter as the
//! host's device, into a texture shared with the host; the host's GPU copies its backbuffer
//! into that texture, waits for the overlay, and copies the result back, ordered by a shared
//! fence. Nothing about the host's pipeline state is touched (copies and fence operations are
//! not part of it), and the host's thread never waits for the GPU.
//!
//! **Not run.** This path was written against the Win32 documentation and the wgpu-hal
//! sources and compiles for `x86_64-pc-windows-msvc`; it has not been run on Windows.

mod hooks;
mod host11;
mod host12;
mod probe;
mod shared;
mod state;
mod vtable;
mod wgpu_side;

use crate::{core::Core, error::HookError, options::Api};
use std::sync::Arc;

pub(crate) fn install(core: &Arc<Core>, _api: Api) -> Result<(), HookError> {
    let _ = core;
    hooks::install().map_err(HookError::Hook)
}

pub(crate) fn remove(_core: &Arc<Core>) {
    hooks::remove();
}
