//! Files that arrive from outside the application: dropped from the system's file manager
//! or chosen in a native dialog.
//!
//! One type, [`PickedFile`], stands for a file on every platform: a path on the desktop,
//! a `File` object in a browser. Code that only uses its name, size, kind and
//! [`Context::read_file`](crate::Context::read_file) behaves the same in both. Paths come
//! from the system and are not trusted: nothing here assumes UTF-8, that a file exists, or
//! that a dropped directory is empty; directories are reported as directories and never
//! walked.

mod error;
mod filter;
pub(crate) mod io;
mod mime;
mod picked;
mod task;

pub use error::FileError;
pub use filter::FileFilter;
pub use picked::PickedFile;
pub use task::FileTask;

pub(crate) use task::{Notify, TaskSender};

#[cfg(feature = "file-dialogs")]
mod dialog;
#[cfg(feature = "file-dialogs")]
pub(crate) use dialog::DialogInbox;
#[cfg(feature = "file-dialogs")]
pub use dialog::{
    DialogBackend, DialogLaunch, DialogReply, DialogRequest, DialogResult, FileDialog,
    FileDialogKind, MemoryDialogs, SystemDialogs,
};
