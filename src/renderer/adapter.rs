//! Choosing the instance, surface and adapter, and the limits asked of the device. The
//! desktop tries platform backends; a browser tries WebGPU first and WebGL2 second.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

#[cfg(not(target_arch = "wasm32"))]
pub(super) use native::select;
#[cfg(target_arch = "wasm32")]
pub(super) use web::select;

/// The floor of the limits requested from the device. WebGL2 cannot offer more than its own
/// downlevel set; everything else asks for the common downlevel baseline. The caller raises
/// either to the adapter's real texture resolution.
pub(super) fn baseline_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
    if cfg!(target_arch = "wasm32") && adapter.get_info().backend == wgpu::Backend::Gl {
        wgpu::Limits::downlevel_webgl2_defaults()
    } else {
        wgpu::Limits::downlevel_defaults()
    }
}
