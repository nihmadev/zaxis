//! Files dragged in from the operating system (or the browser).
//!
//! Events arrive between passes and are kept as plain data: the files being dragged over
//! the window (`hovered`), and the files dropped since the last pass (`queued`). A pass
//! starts by resolving which [`DropTarget`](crate::DropTarget) the hover and the drop belong
//! to from the hit regions of the pass before, exactly one for each, and ends by forgetting
//! the drop: files nobody took are gone, they never pile up between frames. Every window's
//! context has its own state, so a drop reaches only the window it was dropped on.

mod events;
mod resolve;

use super::{Context, Id};
use crate::{
    context::drag::state::Hover,
    files::{FileFilter, PickedFile},
    Vec2,
};
use std::sync::Arc;

/// Registered by a target that takes files, on every pass in which it is enabled. Hits
/// refer to it by index, like drag targets.
#[derive(Clone)]
pub(crate) struct FileTarget {
    /// Scoped id, also the id of the passive hit.
    pub id: Id,
    /// The application's id, reported in results.
    pub key: Id,
    pub depth: u16,
    pub filter: Arc<FileFilter>,
    pub max: usize,
    /// A rejecting target hands the drop to its enclosing target.
    pub passthrough: bool,
}

#[derive(Default)]
pub(crate) struct FileDrop {
    /// Being dragged over the window now, in the order the system listed them.
    hovered: Vec<PickedFile>,
    hover_pos: Option<Vec2>,
    /// Dropped since the last pass.
    queued: Vec<PickedFile>,
    queued_pos: Option<Vec2>,
    /// Dropped files this pass can see, until a target or the application takes them.
    dropped: Vec<PickedFile>,
    drop_pos: Option<Vec2>,
    /// Who the hover and the drop belong to, decided when the pass began.
    pub(crate) hover_target: Option<Hover>,
    pub(crate) drop_target: Option<Hover>,
    pub(crate) claimed: bool,
    pub(crate) targets: Vec<FileTarget>,
    last_targets: Vec<FileTarget>,
}

impl Context {
    /// The files being dragged over this window, in the order the system listed them; empty
    /// when nothing is. Platforms that do not announce a drag before the drop (some Linux
    /// compositors) never list any, and a browser lists them without names. While a modal
    /// is open, code outside it sees none.
    pub fn hovered_files(&self) -> &[PickedFile] {
        if self.input_blocked() {
            return &[];
        }
        &self.files.hovered
    }

    /// Where the files being dragged are over the window, in logical pixels. The browser
    /// reports it exactly; the desktop reports the last known pointer position, which is
    /// `None` when the drag came from outside and the pointer never moved over the window.
    pub fn hovered_files_position(&self) -> Option<Vec2> {
        self.files
            .hover_pos
            .filter(|_| !self.hovered_files().is_empty())
    }

    /// The files dropped on this window since the last pass, once. Files that landed on a
    /// [`DropTarget`](crate::DropTarget) that takes files belong to it and are not returned
    /// here; the rest are gone after this pass if nobody takes them. Empty while a modal is
    /// open.
    pub fn take_dropped_files(&mut self) -> Vec<PickedFile> {
        if self.input_blocked() || (self.files.drop_target.is_some() && !self.files.claimed) {
            return Vec::new();
        }
        std::mem::take(&mut self.files.dropped)
    }

    /// The files dropped this pass, without taking them.
    pub fn dropped_files(&self) -> &[PickedFile] {
        if self.input_blocked() {
            return &[];
        }
        &self.files.dropped
    }

    /// Where the files of this pass were dropped; see [`hovered_files_position`](Self::hovered_files_position).
    pub fn dropped_files_position(&self) -> Option<Vec2> {
        self.files.drop_pos
    }

    /// Start of a pass: the queued drop becomes this pass's, and the hover and the drop find
    /// their targets among the regions the previous pass published.
    pub(crate) fn file_drop_begin(&mut self) {
        self.files.targets.clear();
        self.files.claimed = false;
        if !self.files.queued.is_empty() {
            self.files.dropped = std::mem::take(&mut self.files.queued);
            self.files.drop_pos = self.files.queued_pos.take();
        }
        self.files.hover_target = self.file_hover_target();
        self.files.drop_target = self.file_drop_target();
    }

    /// End of a pass: the registry becomes the one events resolve against, and a drop that
    /// nobody took is gone.
    pub(crate) fn file_drop_finish(&mut self) {
        std::mem::swap(&mut self.files.last_targets, &mut self.files.targets);
        self.files.targets.clear();
        self.files.dropped.clear();
        self.files.drop_target = None;
        self.files.drop_pos = None;
    }

    pub(crate) fn file_add_target(&mut self, target: FileTarget) -> u32 {
        self.files.targets.push(target);
        (self.files.targets.len() - 1) as u32
    }
}
