//! The system's dialogs through `rfd`: Win32 and Cocoa dialogs, the xdg-desktop-portal on
//! Linux (no GTK needed), `<input type=file>` in a browser. Each dialog runs off the
//! frame: on a thread of its own on the desktop, as a task in a browser.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

use super::{DialogBackend, DialogReply, FileDialog};
use std::sync::Arc;

/// The platform's own file dialogs.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemDialogs;

impl DialogBackend for SystemDialogs {
    fn launch(
        &mut self,
        dialog: &FileDialog,
        parent: Option<&Arc<winit::window::Window>>,
        reply: DialogReply,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        native::launch(dialog, parent, reply);
        #[cfg(target_arch = "wasm32")]
        {
            let _ = parent;
            web::launch(dialog, reply);
        }
    }
}

/// An `rfd` dialog carrying everything of `dialog` except its parent window.
fn configure(dialog: &FileDialog) -> rfd::AsyncFileDialog {
    let mut builder = rfd::AsyncFileDialog::new();
    if let Some(title) = dialog.title_text() {
        builder = builder.set_title(title);
    }
    if let Some(directory) = dialog.start_directory() {
        builder = builder.set_directory(directory);
    }
    if let Some(name) = dialog.default_file_name() {
        builder = builder.set_file_name(name);
    }
    for filter in dialog.filters() {
        builder = builder.add_filter(filter.name(), filter.extensions());
    }
    builder
}
