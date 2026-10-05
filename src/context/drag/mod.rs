//! In-window drag and drop runtime.
//!
//! A press on a source only *arms* a drag; the source and whatever is under the
//! press keep clicks and hover until the pointer travels the logical threshold.
//! The session then takes over the existing pointer capture, so it follows the
//! pointer outside the source, the Window and the native window, and ends
//! exactly once: release, Escape, focus loss, resize, or a lost source.
//!
//! The payload is owned by the session as `Box<dyn Any>` with its `TypeId`.
//! Targets compare the type, then call the application's predicate on the
//! value, before highlighting and again before accepting. Nothing is serialized
//! and no closure is stored; per-pass registries are plain data.
#[doc(hidden)]
pub mod autoscroll;
mod frame;
mod input;
mod keyboard;
pub mod preview;
pub mod state;
mod targets;

use super::{Context, Id};
use state::{SourceInfo, TargetInfo};

pub(crate) use state::{DragRuntime, Payload};

impl Context {
    /// Nesting depth of the component being built, for target priority.
    pub(crate) fn drag_enter(&mut self) -> u16 {
        let depth = self.drag.depth;
        self.drag.depth = depth.saturating_add(1);
        depth
    }
    pub(crate) fn drag_exit(&mut self) {
        self.drag.depth = self.drag.depth.saturating_sub(1);
    }

    pub(crate) fn drag_add_source(&mut self, info: SourceInfo) -> u32 {
        self.drag.sources.push(info);
        (self.drag.sources.len() - 1) as u32
    }

    pub(crate) fn drag_add_target(&mut self, info: TargetInfo) -> u32 {
        self.drag.targets.push(info);
        (self.drag.targets.len() - 1) as u32
    }

    /// A drag is running and its payload is available to targets.
    pub(crate) fn drag_running(&self) -> bool {
        self.drag.active()
    }

    /// Mark `source` as present on this pass. Returns the session when it is
    /// the dragged source, so the caller can hand over its payload.
    pub(crate) fn drag_source_seen(&mut self, source: Id) -> Option<&mut state::Session> {
        let session = self
            .drag
            .session
            .as_mut()
            .filter(|session| session.info.id == source)?;
        session.seen = true;
        Some(session)
    }
}

impl Context {
    /// Keep the dragged source alive by application key when its widget is not
    /// built, for example a virtualized row scrolled out of the viewport.
    pub(crate) fn drag_keep_alive(&mut self, key: Id) {
        if let Some(session) = self.drag.session.as_mut().filter(|s| s.info.key == key) {
            session.seen = true;
        }
    }
}

impl Context {
    /// Claim a hit's place in the stacking order before its rectangle is known.
    /// Registering the hit later keeps this position, so a region wrapped around
    /// content that is built first still sits below that content's own hits.
    pub(crate) fn drag_reserve_order(&mut self, id: Id) {
        self.interaction.reserve(id);
    }
}
