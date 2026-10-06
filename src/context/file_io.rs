//! Reading and writing picked files without blocking the frame.

use super::Context;
use crate::files::{io, FileTask, PickedFile, TaskSender};

impl Context {
    /// Read the whole of `file` in the background. Poll the task each frame with
    /// [`FileTask::take`]: the frame it finishes in is drawn once, the result is delivered
    /// once, and a directory, a vanished file or a refused read comes back as an error.
    pub fn read_file(&mut self, file: &PickedFile) -> FileTask<Vec<u8>> {
        let (sender, task) = TaskSender::new(self.file_notify.clone());
        io::read(file, sender);
        task
    }

    /// Write `bytes` to `file` in the background: over the file on the desktop, as a
    /// download named like it in a browser (the file a save dialog answered with).
    pub fn write_file(&mut self, file: &PickedFile, bytes: Vec<u8>) -> FileTask<()> {
        let (sender, task) = TaskSender::new(self.file_notify.clone());
        io::write(file, bytes, sender);
        task
    }
}
