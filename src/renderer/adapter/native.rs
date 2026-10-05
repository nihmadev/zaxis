use crate::RenderError;
use std::sync::Arc;
use winit::window::Window;

/// DX12 needs far less driver memory than Vulkan on Windows, so it is tried
/// first. `WGPU_BACKEND` overrides the order; the default set is the fallback.
/// `backends` restricts the attempt to exactly that set.
pub(in crate::renderer) async fn select(
    window: &Arc<Window>,
    backends: Option<wgpu::Backends>,
) -> Result<(wgpu::Instance, wgpu::Surface<'static>, wgpu::Adapter), RenderError> {
    let explicit = backends.is_some() || std::env::var_os("WGPU_BACKEND").is_some();
    let mut attempts =
        vec![backends.unwrap_or_else(|| wgpu::Backends::from_env().unwrap_or_default())];
    if cfg!(windows) && !explicit {
        attempts.insert(0, wgpu::Backends::DX12);
    }
    let mut failure = None;
    for backends in attempts {
        let mut descriptor = wgpu::InstanceDescriptor::new_with_display_handle_from_env(Box::new(
            Arc::clone(window),
        ));
        descriptor.backends = backends;
        let instance = wgpu::Instance::new(descriptor);
        let surface = match instance.create_surface(Arc::clone(window)) {
            Ok(surface) => surface,
            Err(error) => {
                failure = Some(RenderError::CreateSurface(error));
                continue;
            }
        };
        match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                ..Default::default()
            })
            .await
        {
            Ok(adapter) => return Ok((instance, surface, adapter)),
            Err(error) => failure = Some(RenderError::RequestAdapter(error)),
        }
    }
    Err(failure.expect("at least one backend set is tried"))
}
