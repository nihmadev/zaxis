use super::{ClipboardBackend, ClipboardError};

/// The operating system clipboard.
pub(super) struct NativeClipboard(arboard::Clipboard);

impl NativeClipboard {
    pub(super) fn new() -> Result<Self, ClipboardError> {
        arboard::Clipboard::new().map(Self).map_err(convert)
    }
}

impl ClipboardBackend for NativeClipboard {
    fn text(&mut self) -> Result<String, ClipboardError> {
        self.0.get_text().map_err(convert)
    }

    fn set_text(&mut self, text: String) -> Result<(), ClipboardError> {
        self.0.set_text(text).map_err(convert)
    }
}

fn convert(error: arboard::Error) -> ClipboardError {
    match error {
        arboard::Error::ContentNotAvailable => ClipboardError::ContentNotAvailable,
        other => ClipboardError::Unavailable(other.to_string()),
    }
}
