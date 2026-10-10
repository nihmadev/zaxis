//! Draws a zaxis interface over the frames of the process it runs in, by hooking how the host
//! presents them: a debug or profiling overlay, a mod's menu, a tool for a 3D editor.
//!
//! The scope is tools and mods for processes the developer controls or that allow it. This
//! crate gets control only after it has been loaded (a Vulkan implicit layer, a mod loader, a
//! plugin); it does not load itself into other processes, hide, or work around any
//! protection.
//!
//! ```no_run
//! use z_hook::{Overlay, OverlayOptions};
//!
//! let overlay = Overlay::install(OverlayOptions::default(), |ctx| {
//!     zaxis::Window::new("Debug").show(ctx, |ui| {
//!         ui.label("hello from inside the host");
//!     });
//! })
//! .expect("a graphics API to hook");
//! // F10 shows and hides it; dropping `overlay` removes the hooks.
//! # drop(overlay);
//! ```
//!
//! # Threads
//!
//! The interface (`Context`, which is not `Send`) lives on the thread that presents, the
//! host's render thread, and never leaves it. Input arrives on other threads (a window
//! procedure, an X11 reader) through a queue; on the render thread, outside a frame, it is
//! handled at once. Nothing on the render thread waits for the GPU or for another thread.

#![deny(unsafe_op_in_unsafe_fn)]

mod backend;
mod budget;
mod core;
mod driver;
mod dxgi_format;
mod error;
mod input;
pub mod log;
mod options;
mod overlay;
mod registry;
mod stats;
#[doc(hidden)]
pub mod testing;
pub mod win32;

#[cfg(all(feature = "vulkan", any(unix, windows)))]
pub mod vulkan;

#[cfg(windows)]
mod dxgi;

pub use backend::{BackendError, PresentBackend, SurfaceInfo};
pub use driver::{FrameOutcome, SkipReason};
#[doc(hidden)]
pub use dxgi_format::{plan as dxgi_plan, FormatPlan};
pub use error::HookError;
pub use input::{decide, Delivery};
pub use options::{Api, ApiSet, OutputColor, OverlayInput, OverlayOptions, ToggleKey};
pub use overlay::{input_sink, InputSink, Overlay};
pub use stats::OverlayStats;
