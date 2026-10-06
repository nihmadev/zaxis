use super::FileError;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

/// How background work tells the context that something finished: a flag the context
/// reads when it decides to repaint (so one result asks for exactly one frame) and the
/// host's waker, the same one that wakes the loop for finished images.
#[derive(Clone)]
pub(crate) struct Notify {
    ready: Arc<AtomicBool>,
    images: crate::images::SharedImages,
}

impl Notify {
    pub(crate) fn new(images: crate::images::SharedImages) -> Self {
        Self {
            ready: Arc::new(AtomicBool::new(false)),
            images,
        }
    }

    /// Something finished: ask for a frame and wake the event loop. Callable from any thread.
    pub(crate) fn signal(&self) {
        self.ready.store(true, Ordering::Release);
        let waker = self.images.lock().waker();
        if let Some(wake) = waker {
            wake();
        }
    }

    pub(crate) fn pending(&self) -> bool {
        self.ready.load(Ordering::Acquire)
    }

    /// A pass begins and will see everything signalled so far.
    pub(crate) fn acknowledge(&self) {
        self.ready.store(false, Ordering::Release);
    }
}

enum Slot<T> {
    Pending,
    Ready(Result<T, FileError>),
    Taken,
}

/// A result that arrives from background work: poll it each frame with
/// [`take`](Self::take). Clones share one result, and the first `take` after it is ready
/// gets it.
pub struct FileTask<T> {
    slot: Arc<Mutex<Slot<T>>>,
}

impl<T> Clone for FileTask<T> {
    fn clone(&self) -> Self {
        Self {
            slot: Arc::clone(&self.slot),
        }
    }
}

impl<T> FileTask<T> {
    /// The result, exactly once, when the work has finished; `None` while it runs or after
    /// the result was taken.
    pub fn take(&self) -> Option<Result<T, FileError>> {
        let mut slot = self.slot.lock().expect("file task");
        match std::mem::replace(&mut *slot, Slot::Taken) {
            Slot::Ready(result) => Some(result),
            Slot::Pending => {
                *slot = Slot::Pending;
                None
            }
            Slot::Taken => None,
        }
    }

    /// The work has not finished.
    pub fn is_pending(&self) -> bool {
        matches!(*self.slot.lock().expect("file task"), Slot::Pending)
    }
}

/// The worker's end of a [`FileTask`].
pub(crate) struct TaskSender<T> {
    slot: Arc<Mutex<Slot<T>>>,
    notify: Notify,
}

impl<T> TaskSender<T> {
    pub(crate) fn new(notify: Notify) -> (Self, FileTask<T>) {
        let slot = Arc::new(Mutex::new(Slot::Pending));
        (
            Self {
                slot: Arc::clone(&slot),
                notify,
            },
            FileTask { slot },
        )
    }

    pub(crate) fn complete(self, result: Result<T, FileError>) {
        *self.slot.lock().expect("file task") = Slot::Ready(result);
        self.notify.signal();
    }
}
