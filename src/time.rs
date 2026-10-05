//! The clock behind every deadline in the crate.
//!
//! `Instant` is `std::time::Instant` on native targets, so native signatures such as
//! [`Context::run_at`](crate::Context::run_at) are unchanged. `std::time::Instant::now()`
//! panics on `wasm32-unknown-unknown`, so there it is `web_time::Instant`, the same type
//! winit uses for `ControlFlow::WaitUntil`. `Duration` is the std type on both.

#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;
#[cfg(target_arch = "wasm32")]
pub use web_time::Instant;
