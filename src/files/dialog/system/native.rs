use super::{configure, DialogReply, FileDialog};
use crate::files::{
    dialog::{DialogResult, FileDialogKind},
    FileError, PickedFile,
};
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::Arc,
    thread,
};
use winit::window::Window;

/// Show the dialog on a thread of its own and answer from it. `rfd` runs the dialog where
/// the platform wants it (the main queue on macOS); the thread only waits.
pub(super) fn launch(dialog: &FileDialog, parent: Option<&Arc<Window>>, reply: DialogReply) {
    let (dialog, parent, thread_reply) = (dialog.clone(), parent.cloned(), reply.clone());
    let started = thread::Builder::new()
        .name("zaxis-file-dialog".into())
        .spawn(move || {
            let shown = catch_unwind(AssertUnwindSafe(|| {
                pollster::block_on(show(&dialog, parent.as_deref()))
            }));
            let result = shown.unwrap_or_else(|_| {
                DialogResult::Failed(FileError::Platform("the dialog backend panicked".into()))
            });
            thread_reply.send(result);
        });
    if let Err(error) = started {
        reply.send(DialogResult::Failed(FileError::io(&error)));
    }
}

async fn show(dialog: &FileDialog, parent: Option<&Window>) -> DialogResult {
    let mut builder = configure(dialog);
    if let Some(parent) = parent {
        builder = builder.set_parent(parent);
    }
    let handles = match dialog.kind() {
        FileDialogKind::OpenFile => builder.pick_file().await.map(|h| vec![h]),
        FileDialogKind::OpenFiles => builder.pick_files().await,
        FileDialogKind::SaveFile => builder.save_file().await.map(|h| vec![h]),
        FileDialogKind::PickFolder => builder.pick_folder().await.map(|h| vec![h]),
        FileDialogKind::PickFolders => builder.pick_folders().await,
    };
    match handles {
        Some(handles) if !handles.is_empty() => DialogResult::Picked(
            handles
                .iter()
                .map(|handle| PickedFile::from_path(handle.path()))
                .collect(),
        ),
        _ => DialogResult::Cancelled,
    }
}
