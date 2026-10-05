//! The platform adapter of one native window: UI Automation on Windows, NSAccessibility on
//! macOS, AT-SPI on Linux and the BSDs with the `accesskit_unix` feature (nothing without).
//!
//! The adapter is created with the window, before the window is first shown, and lives as
//! long as the window does: a renderer rebuilt after a lost device keeps it. Assistive
//! technology is served lazily. Until it asks for the tree, the adapter holds a
//! placeholder and the context collects nothing; `InitialTreeRequested` then turns
//! collection on and the next frame publishes the whole tree. Returning the tree from the
//! request itself would mean keeping a copy of it ready at all times, which is exactly the
//! cost laziness avoids, so the request is answered one frame later instead.

use crate::Context;
use accesskit_winit::WindowEvent;
use winit::{
    error::OsError,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowAttributes},
};

pub(crate) use accesskit_winit::Event;

pub(crate) struct Adapter(accesskit_winit::Adapter);

/// Create a window together with its adapter. The adapter has to exist before the window
/// is visible, so the window is created hidden and shown afterwards when `attributes` asks
/// for a visible one. With `enabled` off this is plain window creation.
pub(crate) fn create_window<T: From<Event> + Send + 'static>(
    event_loop: &ActiveEventLoop,
    attributes: WindowAttributes,
    proxy: Option<EventLoopProxy<T>>,
) -> Result<(Window, Option<Adapter>), OsError> {
    let Some(proxy) = proxy else {
        return Ok((event_loop.create_window(attributes)?, None));
    };
    let visible = attributes.visible;
    let window = event_loop.create_window(attributes.with_visible(false))?;
    let adapter = Adapter(accesskit_winit::Adapter::with_event_loop_proxy(
        event_loop, &window, proxy,
    ));
    if visible {
        window.set_visible(true);
    }
    Ok((window, Some(adapter)))
}

impl Adapter {
    /// Every window event passes through here before the context sees it: the adapter
    /// tracks focus and bounds of the native window itself.
    pub(crate) fn process_event(&mut self, window: &Window, event: &winit::event::WindowEvent) {
        self.0.process_event(window, event);
    }

    /// After a pass: hand the changes of the tree to assistive technology, if any.
    pub(crate) fn publish(&mut self, context: &mut Context) {
        // An update the adapter does not take stays with the context, which then sends
        // the whole tree next time: nothing is lost between the two.
        if context.accessibility_update_pending() {
            self.0.update_if_active(|| {
                context
                    .take_accessibility_update()
                    .expect("an update is pending")
            });
        }
    }
}

/// Apply an adapter event to the window's context. Returns whether the window needs a
/// frame: to build the tree that was asked for, or to show what a request changed.
pub(crate) fn handle(context: &mut Context, event: WindowEvent) -> bool {
    match event {
        WindowEvent::InitialTreeRequested => {
            // The adapter starts over from a placeholder: whatever was sent before is void.
            context.set_accessibility_active(false);
            context.set_accessibility_active(true);
            true
        }
        WindowEvent::ActionRequested(request) => context.on_accessibility_action(&request).repaint,
        WindowEvent::AccessibilityDeactivated => {
            context.set_accessibility_active(false);
            false
        }
    }
}
