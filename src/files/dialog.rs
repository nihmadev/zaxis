//! Native file dialogs: a [`FileDialog`] describes one,
//! [`Context::open_dialog`](crate::Context::open_dialog) starts it, and the answer is polled
//! with [`Context::take_dialog_result`](crate::Context::take_dialog_result).
//!
//! The dialog itself is shown by a [`DialogBackend`]: the system's (through `rfd`) in the
//! runner, an in-memory one in tests. The frame never waits for it.

mod backend;
mod memory;
mod request;
mod spec;
mod system;

pub use backend::{DialogBackend, DialogLaunch, DialogReply};
pub use memory::MemoryDialogs;
pub use request::{DialogRequest, DialogResult};
pub use spec::{FileDialog, FileDialogKind};
pub use system::SystemDialogs;

pub(crate) use backend::DialogInbox;
