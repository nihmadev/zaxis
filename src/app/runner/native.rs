//! A native window with its renderer, and creating one.

use crate::app::{WindowError, WindowInfo, WindowOptions};
use crate::{Renderer, SharedResources, Vec2};
use std::sync::Arc;
use winit::{
    event_loop::ActiveEventLoop,
    window::{CursorIcon, Window},
};

/// Renderer first: it is dropped before the window it presents to.
pub(in crate::app) struct Native {
    pub(in crate::app) renderer: Renderer,
    pub(in crate::app) window: Arc<Window>,
    pub(in crate::app) occluded: bool,
    pub(in crate::app) retry_at: Option<std::time::Instant>,
    pub(in crate::app) cursor: CursorIcon,
    pub(in crate::app) transparent: bool,
}

impl Native {
    /// Rendering is pointless for windows that are covered, minimized, hidden or empty.
    pub(in crate::app) fn visible(&self) -> bool {
        let size = self.window.inner_size();
        !self.occluded
            && self.window.is_minimized() != Some(true)
            && self.window.is_visible() != Some(false)
            && size.width > 0
            && size.height > 0
    }

    pub(in crate::app) fn info(&self) -> WindowInfo {
        let size = self.window.inner_size();
        let scale_factor = self.window.scale_factor();
        WindowInfo {
            size,
            logical_size: Vec2::new(size.width as f32, size.height as f32)
                / scale_factor as f32,
            scale_factor,
            focused: self.window.has_focus(),
            minimized: self.window.is_minimized() == Some(true),
            maximized: self.window.is_maximized(),
            fullscreen: self.window.fullscreen().is_some(),
            occluded: self.occluded,
            visible: self.window.is_visible() != Some(false),
        }
    }
}

/// Create the native window and its renderer. `sibling` is any existing renderer: the new
/// window then shares its device, pipelines and textures instead of starting a new device.
pub(in crate::app) fn create(
    event_loop: &ActiveEventLoop,
    options: &WindowOptions,
    owner: Option<&Window>,
    sibling: Option<&Renderer>,
    default_mode: crate::PresentationMode,
    resources: &SharedResources,
) -> Result<Native, WindowError> {
    let attributes = options.attributes.clone();
    let transparent = attributes.transparent;
    let decorations = cfg!(target_os = "macos") || attributes.decorations;
    let attributes = attributes.with_decorations(decorations);
    let window = Arc::new(
        event_loop
            .create_window(owned(attributes, owner))
            .map_err(WindowError::Create)?,
    );
    let mode = options.presentation_mode.unwrap_or(default_mode);
    let mut renderer = match sibling {
        Some(sibling) => sibling.create_sibling(Arc::clone(&window), mode),
        None => pollster::block_on(Renderer::new_with_presentation_mode(
            Arc::clone(&window),
            mode,
        )),
    }
    .map_err(WindowError::Render)?;
    renderer.set_transparent(transparent);
    clamp_image_limits(resources, &renderer);
    if window.is_visible() != Some(false) {
        window.request_redraw();
    }
    Ok(Native {
        renderer,
        window,
        occluded: false,
        retry_at: None,
        cursor: CursorIcon::Default,
        transparent,
    })
}

/// Images larger than the GPU can hold are rejected up front, for every window.
pub(in crate::app) fn clamp_image_limits(resources: &SharedResources, renderer: &Renderer) {
    let mut limits = resources.image_limits();
    let max = limits.max_dimension.min(renderer.max_texture_dimension_2d());
    if max != limits.max_dimension {
        limits.max_dimension = max;
        resources.set_image_limits(limits);
    }
}

/// An owned window stays above its owner in the Z-order and is destroyed with it.
#[cfg(target_os = "windows")]
fn owned(
    attributes: winit::window::WindowAttributes,
    owner: Option<&Window>,
) -> winit::window::WindowAttributes {
    use winit::{
        platform::windows::WindowAttributesExtWindows,
        raw_window_handle::{HasWindowHandle, RawWindowHandle},
    };
    match owner.map(|owner| owner.window_handle().map(|handle| handle.as_raw())) {
        Some(Ok(RawWindowHandle::Win32(handle))) => attributes.with_owner_window(handle.hwnd.get()),
        _ => attributes,
    }
}

/// Other platforms leave the Z-order of a child to the system; the runner still closes it
/// together with its parent.
#[cfg(not(target_os = "windows"))]
fn owned(
    attributes: winit::window::WindowAttributes,
    _owner: Option<&Window>,
) -> winit::window::WindowAttributes {
    attributes
}
