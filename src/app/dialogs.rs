//! Which dialogs are open and for which window, independent of winit and the system, so the
//! rules (parent modality, cancelling with a closing window) can be tested with a recording
//! backend.

use super::WindowKey;
use crate::files::{DialogBackend, DialogLaunch, DialogReply, DialogResult, SystemDialogs};
use std::sync::Arc;
use winit::window::Window;

struct Running {
    launcher: WindowKey,
    parent: WindowKey,
    reply: DialogReply,
}

/// Shows the dialogs the windows open and tracks them until they are answered.
pub struct DialogHost {
    backend: Box<dyn DialogBackend>,
    running: Vec<Running>,
}

impl DialogHost {
    pub fn new(backend: impl DialogBackend + 'static) -> Self {
        Self {
            backend: Box::new(backend),
            running: Vec::new(),
        }
    }

    /// The platform's dialogs.
    pub fn system() -> Self {
        Self::new(SystemDialogs)
    }

    /// Show a dialog `launcher` opened, modal for `parent` (whose native window is `window`).
    pub fn launch(
        &mut self,
        launcher: &WindowKey,
        launch: DialogLaunch,
        parent: &WindowKey,
        window: Option<&Arc<Window>>,
    ) {
        self.running.retain(|r| !r.reply.is_answered());
        self.running.push(Running {
            launcher: launcher.clone(),
            parent: parent.clone(),
            reply: launch.reply.clone(),
        });
        self.backend.launch(&launch.dialog, window, launch.reply);
    }

    /// Windows closed: a dialog that belonged to one of them is answered `Cancelled`, and
    /// nothing is kept for a window that no longer exists.
    pub fn window_closed(&mut self, closed: &[WindowKey]) {
        self.running.retain(|running| {
            if closed.contains(&running.parent) {
                running.reply.send(DialogResult::Cancelled);
            }
            !running.reply.is_answered()
                && !closed.contains(&running.parent)
                && !closed.contains(&running.launcher)
        });
    }

    /// Dialogs still waiting for an answer.
    pub fn open(&mut self) -> usize {
        self.running.retain(|r| !r.reply.is_answered());
        self.running.len()
    }
}
