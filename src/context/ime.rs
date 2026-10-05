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
}
