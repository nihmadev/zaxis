//! Native file dialogs opened from the UI. The context records the request; whoever owns
//! the native window shows it ([`DialogBackend`](crate::DialogBackend)) and answers through
//! the reply, which wakes the loop once. Nothing here waits.

use super::{Context, Id};
use crate::files::{DialogLaunch, DialogReply, DialogRequest, DialogResult, FileDialog};
use std::{collections::HashMap, hash::Hash, sync::Arc};

enum Entry {
    Pending,
    Ready(DialogResult),
}

pub(crate) struct Dialogs {
    entries: HashMap<Id, Entry>,
    launches: Vec<DialogLaunch>,
    inbox: Arc<crate::files::DialogInbox>,
}

impl Dialogs {
    pub(crate) fn new(notify: crate::files::Notify) -> Self {
        Self {
            entries: HashMap::new(),
            launches: Vec::new(),
            inbox: crate::files::DialogInbox::new(notify),
        }
    }

    fn settle(&mut self) {
        for (id, result) in self.inbox.drain() {
            if let Some(entry) = self.entries.get_mut(&id) {
                *entry = Entry::Ready(result);
            }
        }
    }
}

impl Context {
    /// Open `dialog`, identified by `key`. Nothing blocks: the dialog runs on its own and
    /// [`take_dialog_result`](Self::take_dialog_result) has the answer on a later frame.
    ///
    /// A key that is still open or whose answer is not taken yet returns the same handle,
    /// so a button pressed twice opens one dialog. Once the result is taken the key can
    /// open a new one.
    pub fn open_dialog(&mut self, key: impl Hash, dialog: FileDialog) -> DialogRequest {
        let id = Id::new(("file-dialog", key));
        let request = DialogRequest { id };
        if self.dialogs.entries.contains_key(&id) {
            return request;
        }
        self.dialogs.entries.insert(id, Entry::Pending);
        let reply = DialogReply::new(id, Arc::clone(&self.dialogs.inbox));
        self.dialogs.launches.push(DialogLaunch { dialog, reply });
        request
    }

    /// The answer of a dialog, exactly once, when it has ended; `None` while it is open and
    /// after the answer was taken.
    pub fn take_dialog_result(&mut self, request: &DialogRequest) -> Option<DialogResult> {
        self.dialogs.settle();
        match self.dialogs.entries.get(&request.id)? {
            Entry::Pending => None,
            Entry::Ready(_) => match self.dialogs.entries.remove(&request.id) {
                Some(Entry::Ready(result)) => Some(result),
                _ => None,
            },
        }
    }

    /// The dialog is open, or answered and not taken yet.
    pub fn dialog_pending(&mut self, request: &DialogRequest) -> bool {
        self.dialogs.settle();
        self.dialogs.entries.contains_key(&request.id)
    }

    /// Dialogs opened since the last call, for a custom host to show with a
    /// [`DialogBackend`](crate::DialogBackend). The runner does this itself.
    pub fn take_dialog_launches(&mut self) -> Vec<DialogLaunch> {
        std::mem::take(&mut self.dialogs.launches)
    }

    /// Answer a dialog as the user would, without any window: for tests.
    pub(crate) fn dialog_answer(&mut self, request: &DialogRequest, result: DialogResult) {
        if let Some(entry) = self.dialogs.entries.get_mut(&request.id) {
            *entry = Entry::Ready(result);
            self.request_repaint();
        }
    }
}
