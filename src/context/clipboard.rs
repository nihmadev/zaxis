//! The text clipboard behind Ctrl+C/X/V, copy buttons and link menus.
//!
//! Widgets call [`Context::copy_text`] and [`Context::clipboard_text`]; what answers is a
//! [`ClipboardBackend`]: the system clipboard on desktop, the document's copy/paste events
//! and `navigator.clipboard` in a browser, or whatever a host installs with
//! [`Context::set_clipboard`]. Failures other than "no text" reach [`Context::diagnostics`].

mod memory;
#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;

use super::Context;
use std::{error::Error, fmt};

pub use memory::MemoryClipboard;

/// Why the clipboard could not be read or written.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClipboardError {
    /// The clipboard holds no text. Normal, never reported.
    ContentNotAvailable,
    /// The system or browser refused: no clipboard service, missing permission, an
    /// insecure page, or a request outside a user gesture.
    Unavailable(String),
}

impl fmt::Display for ClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ContentNotAvailable => f.write_str("the clipboard holds no text"),
            Self::Unavailable(reason) => f.write_str(reason),
        }
    }
}

impl Error for ClipboardError {}

/// A source and sink of plain text for one [`Context`].
pub trait ClipboardBackend {
    /// The text a paste should insert now.
    fn text(&mut self) -> Result<String, ClipboardError>;

    /// Replace the clipboard text.
    fn set_text(&mut self, text: String) -> Result<(), ClipboardError>;

    /// A write that failed after [`set_text`](Self::set_text) returned, as browsers complete
    /// writes asynchronously. Polled once per frame; the default never fails late.
    fn take_error(&mut self) -> Option<ClipboardError> {
        None
    }
}

fn system() -> Result<Box<dyn ClipboardBackend>, ClipboardError> {
    #[cfg(not(target_arch = "wasm32"))]
    return native::NativeClipboard::new().map(|c| Box::new(c) as _);
    #[cfg(target_arch = "wasm32")]
    return Ok(Box::new(web::WebClipboard));
}

impl Context {
    /// Use `backend` instead of the system clipboard, for example in tests or on a host
    /// with its own clipboard service.
    pub fn set_clipboard(&mut self, backend: impl ClipboardBackend + 'static) {
        self.clipboard = Some(Box::new(backend));
    }

    pub(crate) fn clipboard_text(&mut self) -> Result<String, ClipboardError> {
        let result = self.clipboard().and_then(|clipboard| clipboard.text());
        self.report_clipboard(result.as_ref().err());
        result
    }

    pub(crate) fn copy_text(&mut self, text: String) -> Result<(), ClipboardError> {
        let result = self
            .clipboard()
            .and_then(|clipboard| clipboard.set_text(text));
        self.report_clipboard(result.as_ref().err());
        result
    }

    /// Surface writes that failed after returning, once per frame.
    pub(super) fn poll_clipboard(&mut self) {
        let error = self.clipboard.as_mut().and_then(|c| c.take_error());
        self.report_clipboard(error.as_ref());
    }

    fn clipboard(&mut self) -> Result<&mut (dyn ClipboardBackend + 'static), ClipboardError> {
        if self.clipboard.is_none() {
            self.clipboard = Some(system()?);
        }
        Ok(self
            .clipboard
            .as_deref_mut()
            .expect("clipboard backend set"))
    }

    fn report_clipboard(&mut self, error: Option<&ClipboardError>) {
        if let Some(error) = error.filter(|e| **e != ClipboardError::ContentNotAvailable) {
            self.report(crate::DiagnosticKind::External, None, None, || {
                format!("clipboard unavailable: {error}")
            });
        }
    }
}
