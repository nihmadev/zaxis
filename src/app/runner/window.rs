//! A window with its renderer, on every platform.

use crate::app::{WindowInfo, WindowOptions};
use crate::time::Instant;
use crate::{Renderer, SharedResources, Vec2};
use std::sync::Arc;
use winit::window::{CursorIcon, Window};

/// Renderer first: it is dropped before the window it presents to.
pub(in crate::app) struct Native {
    pub(in crate::app) renderer: Renderer,
    /// Serves the accessibility tree of the window; dropped before the window it is
    /// attached to and kept when only the renderer is rebuilt.
    #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
    pub(in crate::app) adapter: Option<crate::accessibility::adapter::Adapter>,
    pub(in crate::app) window: Arc<Window>,
    pub(in crate::app) occluded: bool,
    pub(in crate::app) retry_at: Option<Instant>,
    pub(in crate::app) cursor: CursorIcon,
    pub(in crate::app) transparent: bool,
    /// A replacement renderer is being created after the device was lost, which the browser
    /// does asynchronously. Nothing is drawn until it arrives.
    pub(in crate::app) rebuilding: bool,
}

impl Native {
    pub(in crate::app) fn new(
        renderer: Renderer,
        window: Arc<Window>,
        options: &WindowOptions,
    ) -> Self {
        Self {
            renderer,
            #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
            adapter: None,
            window,
            occluded: false,
            retry_at: None,
            cursor: CursorIcon::Default,
            transparent: options.attributes.transparent,
            rebuilding: false,
        }
    }

    /// Rendering is pointless for windows that are covered, minimized, hidden or empty.
    pub(in crate::app) fn visible(&self) -> bool {
        let size = self.window.inner_size();
        !self.occluded
            && !self.rebuilding
            && self.window.is_minimized() != Some(true)
            && self.window.is_visible() != Some(false)
            && size.width > 0
            && size.height > 0
    }

    /// The accessibility adapter sees every window event before anything else handles it.
    pub(in crate::app) fn accessibility_event(&mut self, event: &winit::event::WindowEvent) {
        #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
        if let Some(adapter) = &mut self.adapter {
            adapter.process_event(&self.window, event);
        }
        #[cfg(not(all(feature = "accesskit", not(target_arch = "wasm32"))))]
        let _ = event;
    }

    /// After a pass: send what changed in the accessibility tree, when anyone listens.
    pub(in crate::app) fn publish_accessibility(&mut self, context: &mut crate::Context) {
        #[cfg(all(feature = "accesskit", not(target_arch = "wasm32")))]
        if let Some(adapter) = &mut self.adapter {
            adapter.publish(context);
        }
        #[cfg(not(all(feature = "accesskit", not(target_arch = "wasm32"))))]
        let _ = context;
    }

    pub(in crate::app) fn info(&self) -> WindowInfo {
        let size = self.window.inner_size();
        let scale_factor = self.window.scale_factor();
        WindowInfo {
            size,
            logical_size: Vec2::new(size.width as f32, size.height as f32) / scale_factor as f32,
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

/// Images larger than the GPU can hold are rejected up front, for every window.
pub(in crate::app) fn clamp_image_limits(resources: &SharedResources, renderer: &Renderer) {
    let mut limits = resources.image_limits();
    let max = limits
        .max_dimension
        .min(renderer.max_texture_dimension_2d());
    if max != limits.max_dimension {
        limits.max_dimension = max;
        resources.set_image_limits(limits);
    }
}
