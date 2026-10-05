use crate::RenderError;
use std::sync::Arc;
use winit::window::Window;

type Found = (wgpu::Instance, wgpu::Surface<'static>, wgpu::Adapter);

fn instance(backends: wgpu::Backends) -> wgpu::Instance {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = backends;
    wgpu::Instance::new(descriptor)
}

fn options<'a>(
    surface: Option<&'a wgpu::Surface<'static>>,
) -> wgpu::RequestAdapterOptions<'a, 'static> {
    wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: surface,
        force_fallback_adapter: false,
        ..Default::default()
    }
}

/// WebGPU is probed without touching the canvas: a canvas that already handed out a
/// `webgpu` context can never hand out `webgl2`, which would end the fallback.
async fn webgpu(window: &Arc<Window>) -> Result<Found, RenderError> {
    let instance = instance(wgpu::Backends::BROWSER_WEBGPU);
    let adapter = instance
        .request_adapter(&options(None))
        .await
        .map_err(RenderError::RequestAdapter)?;
    let surface = instance
        .create_surface(Arc::clone(window))
        .map_err(RenderError::CreateSurface)?;
    Ok((instance, surface, adapter))
}

/// WebGL2 learns its context from the surface, so the surface comes first.
async fn gl(window: &Arc<Window>) -> Result<Found, RenderError> {
    let instance = instance(wgpu::Backends::GL);
    let surface = instance
        .create_surface(Arc::clone(window))
        .map_err(RenderError::CreateSurface)?;
    let adapter = instance
        .request_adapter(&options(Some(&surface)))
        .await
        .map_err(RenderError::RequestAdapter)?;
    Ok((instance, surface, adapter))
}

/// Try WebGPU, then WebGL2, among the `backends` asked for (both by default).
pub(in crate::renderer) async fn select(
    window: &Arc<Window>,
    backends: Option<wgpu::Backends>,
) -> Result<Found, RenderError> {
    let wanted = backends.unwrap_or(wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL);
    let mut failure = None;
    if wanted.contains(wgpu::Backends::BROWSER_WEBGPU) {
        match webgpu(window).await {
            Ok(found) => return Ok(found),
            Err(error) => failure = Some(error),
        }
    }
    if wanted.contains(wgpu::Backends::GL) {
        match gl(window).await {
            Ok(found) => return Ok(found),
            Err(error) => failure = Some(error),
        }
    }
    Err(failure.unwrap_or(RenderError::UnsupportedSurface))
}
