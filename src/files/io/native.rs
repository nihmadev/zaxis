use crate::files::{picked::Source, FileError, TaskSender};
use std::{
    sync::{Arc, Mutex},
    thread,
};

pub(super) fn read(source: &Source, sender: TaskSender<Vec<u8>>) {
    let Source::Path(path) = source else {
        return sender.complete(Err(FileError::Unsupported("no path to read".into())));
    };
    let path = path.clone();
    spawn(sender, move |sender| {
        sender.complete(std::fs::read(path).map_err(|e| FileError::io(&e)));
    });
}

pub(super) fn write(source: &Source, _name: &str, bytes: Vec<u8>, sender: TaskSender<()>) {
    let Source::Path(path) = source else {
        return sender.complete(Err(FileError::Unsupported("no path to write".into())));
    };
    let path = path.clone();
    spawn(sender, move |sender| {
        sender.complete(std::fs::write(path, bytes).map_err(|e| FileError::io(&e)));
    });
}

/// Run `work` on its own thread; if no thread can be had the task fails instead of hanging.
fn spawn<T: Send + 'static>(
    sender: TaskSender<T>,
    work: impl FnOnce(TaskSender<T>) + Send + 'static,
) {
    let slot = Arc::new(Mutex::new(Some(sender)));
    let for_thread = Arc::clone(&slot);
    let started = thread::Builder::new()
        .name("zaxis-file-io".into())
        .spawn(move || {
            if let Some(sender) = for_thread.lock().ok().and_then(|mut s| s.take()) {
                work(sender);
            }
        });
    if let Err(error) = started {
        if let Some(sender) = slot.lock().ok().and_then(|mut s| s.take()) {
            sender.complete(Err(FileError::io(&error)));
        }
    }
}
