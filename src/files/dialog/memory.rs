use super::{DialogBackend, DialogReply, DialogResult, FileDialog};
use std::sync::Arc;

/// A backend that shows nothing: it remembers the dialogs it was asked for, and a test
/// answers them. No test needs a real dialog.
#[derive(Default)]
pub struct MemoryDialogs {
    opened: Vec<(FileDialog, DialogReply)>,
}

impl MemoryDialogs {
    pub fn new() -> Self {
        Self::default()
    }

    /// The dialogs shown so far, oldest first.
    pub fn opened(&self) -> impl Iterator<Item = &FileDialog> {
        self.opened.iter().map(|(dialog, _)| dialog)
    }

    /// How many dialogs are still waiting for an answer.
    pub fn pending(&self) -> usize {
        self.opened
            .iter()
            .filter(|(_, reply)| !reply.is_answered())
            .count()
    }

    /// Answer the oldest dialog that is still open. Returns false when there is none.
    pub fn answer(&mut self, result: DialogResult) -> bool {
        self.opened
            .iter()
            .find(|(_, reply)| !reply.is_answered())
            .is_some_and(|(_, reply)| reply.send(result))
    }
}

impl DialogBackend for MemoryDialogs {
    fn launch(
        &mut self,
        dialog: &FileDialog,
        _parent: Option<&Arc<winit::window::Window>>,
        reply: DialogReply,
    ) {
        self.opened.push((dialog.clone(), reply));
    }
}
