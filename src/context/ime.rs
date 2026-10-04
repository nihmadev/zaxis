use super::Context;
use crate::Rect;
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    window::Window,
};

impl Context {
    /// Candidate/composition window anchor in logical pixels, or `None` when IME is disabled.
    pub fn ime_cursor_area(&self) -> Option<Rect> {
        self.ime_area
    }

    /// Custom hosts must call this after `run`. The built-in runner does it automatically.
    pub fn sync_ime(&mut self, window: &Window) {
        let target = self.ime_area.and(self.focused_widget);
        if target != self.ime_target {
            // Native composition belongs to one field, even within a single window.
            if self.ime_target.is_some() {
                window.set_ime_allowed(false);
            }
            if target.is_some() {
                window.set_ime_allowed(true);
            }
            self.ime_target = target;
        }
        if let Some(rect) = self.ime_area {
            window.set_ime_cursor_area(
                LogicalPosition::new(f64::from(rect.min.x), f64::from(rect.min.y)),
                LogicalSize::new(f64::from(rect.size().x), f64::from(rect.size().y)),
            );
        }
    }
    pub(crate) fn clipboard_text(&mut self) -> Result<String, arboard::Error> {
        let result = self.clipboard().and_then(|clipboard| clipboard.get_text());
        self.report_clipboard(result.as_ref().err());
        result
    }
    pub(crate) fn copy_text(&mut self, text: String) -> Result<(), arboard::Error> {
        let result = self
            .clipboard()
            .and_then(|clipboard| clipboard.set_text(text));
        self.report_clipboard(result.as_ref().err());
        result
    }
    fn clipboard(&mut self) -> Result<&mut arboard::Clipboard, arboard::Error> {
        if self.clipboard.is_none() {
            self.clipboard = Some(arboard::Clipboard::new()?);
        }
        Ok(self.clipboard.as_mut().unwrap())
    }
    /// A clipboard that holds no text is normal; anything else reaches `Context::diagnostics`.
    fn report_clipboard(&mut self, error: Option<&arboard::Error>) {
        if let Some(error) = error.filter(|e| !matches!(e, arboard::Error::ContentNotAvailable)) {
            self.report(crate::DiagnosticKind::External, None, None, || {
                format!("clipboard unavailable: {error}")
            });
        }
    }
}
