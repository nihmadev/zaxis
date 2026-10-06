//! Events from the system and the browser as plain state changes. Each one asks for a
//! repaint only when what a frame would show differs: a hover that repeats the same files
//! or moves inside the same target changes nothing.

use crate::{context::Context, files::PickedFile, Vec2};

impl Context {
    /// The system announced one more file being dragged over the window (winit reports one
    /// event per file). Returns whether a frame is needed.
    pub(crate) fn file_hovered(&mut self, file: PickedFile) -> bool {
        if self.files.hovered.is_empty() {
            self.files.hover_pos = self.input.pointer;
        }
        self.files.hovered.push(file);
        true
    }

    /// The drag left the window or was cancelled.
    pub(crate) fn file_hover_cancelled(&mut self) -> bool {
        self.files.hover_pos = None;
        let had = !self.files.hovered.is_empty();
        self.files.hovered.clear();
        self.files.hover_target = None;
        had
    }

    /// One file landed. The drag ends with it; files of one drop gather until the next pass,
    /// in the order they arrived, at the position the drop had.
    pub(crate) fn file_dropped(&mut self, file: PickedFile) -> bool {
        if self.files.queued.is_empty() {
            self.files.queued_pos = self.input.pointer.or(self.files.hover_pos);
        }
        self.files.queued.push(file);
        self.files.hovered.clear();
        self.files.hover_pos = None;
        self.files.hover_target = None;
        true
    }

    /// A browser drag enters or moves over the canvas with `files` and an exact position.
    /// Replaces the list; returns whether a frame is needed (a new list or another target).
    pub(crate) fn file_hover_replace(&mut self, files: Vec<PickedFile>, position: Vec2) -> bool {
        let same = self.files.hovered.len() == files.len()
            && self
                .files
                .hovered
                .iter()
                .zip(&files)
                .all(|(a, b)| a.name() == b.name() && a.mime() == b.mime());
        self.files.hovered = files;
        self.files.hover_pos = Some(position);
        !same || self.file_hover_moved()
    }

    /// A browser drop with an exact position.
    pub(crate) fn file_drop_batch(&mut self, files: Vec<PickedFile>, position: Vec2) -> bool {
        self.files.hovered.clear();
        self.files.hover_target = None;
        self.files.hover_pos = None;
        if files.is_empty() {
            return true;
        }
        self.files.queued_pos = Some(position);
        self.files.queued.extend(files);
        true
    }

    /// The pointer moved while files are being dragged (the desktop reports no position of
    /// its own, so its pointer is the best there is). A repaint only if another target is
    /// now under it.
    pub(crate) fn file_hover_pointer(&mut self, position: Vec2) {
        if self.files.hovered.is_empty() {
            return;
        }
        self.files.hover_pos = Some(position);
        if self.file_hover_moved() {
            self.dirty = true;
        }
    }

    /// Re-resolve the hover; true when its target changed.
    fn file_hover_moved(&mut self) -> bool {
        let target = self.file_hover_target();
        let changed =
            target.map(|t| (t.id, t.accepts)) != self.files.hover_target.map(|t| (t.id, t.accepts));
        self.files.hover_target = target;
        changed
    }
}
