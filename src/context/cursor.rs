//! Native cursor hints from the same clipped hit regions used for pointer input.
use super::{Context, HitAction};
use winit::window::CursorIcon;

impl Context {
    /// System cursor for the current hovered region or captured pointer gesture.
    /// The desktop runner applies it automatically. Custom hosts call
    /// `window.set_cursor(context.cursor_icon())` after events and UI passes.
    pub fn cursor_icon(&self) -> CursorIcon {
        let hit = self.capture.map(|capture| capture.hit).or_else(|| {
            self.input
                .pointer
                .and_then(|pointer| self.hit_test(pointer))
        });
        match hit.map(|hit| hit.action) {
            Some(HitAction::ColumnResize { .. }) => CursorIcon::ColResize,
            Some(HitAction::SplitResize { vertical: false }) => CursorIcon::EwResize,
            Some(HitAction::SplitResize { vertical: true }) => CursorIcon::NsResize,
            Some(HitAction::Resize) => CursorIcon::NwseResize,
            Some(HitAction::TextEdit) => CursorIcon::Text,
            Some(HitAction::DragValue) => CursorIcon::EwResize,
            _ => CursorIcon::Default,
        }
    }
}
