use super::{ClipboardBackend, ClipboardError};
use std::sync::{Arc, Mutex};

/// A clipboard held in memory. Clones share the text, so a test can install one in a
/// [`Context`](crate::Context) and keep another to read what the UI copied or to prepare
/// what it will paste.
#[derive(Clone, Debug, Default)]
pub struct MemoryClipboard {
    text: Arc<Mutex<Option<String>>>,
    failure: Arc<Mutex<Option<String>>>,
}

impl MemoryClipboard {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_text(text: impl Into<String>) -> Self {
        let clipboard = Self::new();
        clipboard.set(text);
        clipboard
    }

    pub fn get(&self) -> Option<String> {
        self.text.lock().expect("clipboard mutex").clone()
    }

    pub fn set(&self, text: impl Into<String>) {
        *self.text.lock().expect("clipboard mutex") = Some(text.into());
    }

    /// Make every following read and write fail with `reason`, as a denied permission would.
    /// `None` restores normal operation.
    pub fn fail_with(&self, reason: Option<&str>) {
        *self.failure.lock().expect("clipboard mutex") = reason.map(str::to_owned);
    }

    fn failure(&self) -> Result<(), ClipboardError> {
        match &*self.failure.lock().expect("clipboard mutex") {
            Some(reason) => Err(ClipboardError::Unavailable(reason.clone())),
            None => Ok(()),
        }
    }
}

impl ClipboardBackend for MemoryClipboard {
    fn text(&mut self) -> Result<String, ClipboardError> {
        self.failure()?;
        self.get().ok_or(ClipboardError::ContentNotAvailable)
    }

    fn set_text(&mut self, text: String) -> Result<(), ClipboardError> {
        self.failure()?;
        self.set(text);
        Ok(())
    }
}
