//! Reading and writing a [`PickedFile`] without blocking the frame: a worker thread on the
//! desktop, a promise in a browser. The result lands in a [`FileTask`](super::FileTask).

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(target_arch = "wasm32")]
mod web;

use super::{picked::Source, FileError, PickedFile, TaskSender};

#[cfg(not(target_arch = "wasm32"))]
use native as platform;
#[cfg(target_arch = "wasm32")]
use web as platform;

pub(crate) fn read(file: &PickedFile, sender: TaskSender<Vec<u8>>) {
    match &file.source {
        Source::Memory(bytes) => sender.complete(Ok(bytes.to_vec())),
        Source::Placeholder => sender.complete(Err(FileError::Unsupported(
            "the file is still being dragged; its content is not available yet".into(),
        ))),
        source => platform::read(source, sender),
    }
}

pub(crate) fn write(file: &PickedFile, bytes: Vec<u8>, sender: TaskSender<()>) {
    match &file.source {
        Source::Memory(_) | Source::Placeholder => sender.complete(Err(FileError::Unsupported(
            "this file cannot be written".into(),
        ))),
        source => platform::write(source, file.name(), bytes, sender),
    }
}
