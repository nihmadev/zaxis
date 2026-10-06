use super::{DialogResult, FileDialog};
use crate::{files::Notify, Id};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// Finished dialogs of one context, waiting to be polled.
pub(crate) struct DialogInbox {
    results: Mutex<Vec<(Id, DialogResult)>>,
    notify: Notify,
}

impl DialogInbox {
    pub(crate) fn new(notify: Notify) -> Arc<Self> {
        Arc::new(Self {
            results: Mutex::new(Vec::new()),
            notify,
        })
    }

    pub(crate) fn drain(&self) -> Vec<(Id, DialogResult)> {
        std::mem::take(&mut *self.results.lock().expect("dialog inbox"))
    }
}

/// Where a backend delivers its answer. Cloneable and sendable to the thread that shows the
/// dialog; only the first [`send`](Self::send) counts.
#[derive(Clone)]
pub struct DialogReply {
    id: Id,
    inbox: Arc<DialogInbox>,
    sent: Arc<AtomicBool>,
}

impl DialogReply {
    pub(crate) fn new(id: Id, inbox: Arc<DialogInbox>) -> Self {
        Self {
            id,
            inbox,
            sent: Arc::default(),
        }
    }

    /// Answer the dialog and wake the event loop. Returns false when it was answered
    /// before, for example cancelled by a closing window.
    pub fn send(&self, result: DialogResult) -> bool {
        if self.sent.swap(true, Ordering::AcqRel) {
            return false;
        }
        self.inbox
            .results
            .lock()
            .expect("dialog inbox")
            .push((self.id, result));
        self.inbox.notify.signal();
        true
    }

    /// Whether an answer was already sent.
    pub fn is_answered(&self) -> bool {
        self.sent.load(Ordering::Acquire)
    }
}

/// A dialog the application opened that nobody has shown yet. Hosts take these from
/// [`Context::take_dialog_launches`](crate::Context::take_dialog_launches) after each frame.
pub struct DialogLaunch {
    pub dialog: FileDialog,
    pub reply: DialogReply,
}

/// Shows dialogs and answers through the [`DialogReply`]: the system's on the desktop and in
/// a browser ([`SystemDialogs`](super::SystemDialogs)), a recording one in tests
/// ([`MemoryDialogs`](super::MemoryDialogs)).
pub trait DialogBackend {
    /// Show `dialog` without blocking the caller and answer `reply` when it ends. `parent`
    /// is the native window the dialog belongs to, when there is one.
    fn launch(
        &mut self,
        dialog: &FileDialog,
        parent: Option<&Arc<winit::window::Window>>,
        reply: DialogReply,
    );
}
