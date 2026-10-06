use super::{configure, DialogReply, FileDialog};
use crate::files::{
    dialog::{DialogResult, FileDialogKind},
    FileError, PickedFile,
};

/// A browser cannot show a save dialog or a folder chooser to a page. Open dialogs are
/// `<input type=file>` and need a user gesture; a save answers at once with the name to
/// download as, and [`Context::write_file`](crate::Context::write_file) starts the download.
pub(super) fn launch(dialog: &FileDialog, reply: DialogReply) {
    match dialog.kind() {
        FileDialogKind::SaveFile => {
            let name = dialog.default_file_name().unwrap_or("download").to_owned();
            reply.send(DialogResult::Picked(vec![PickedFile::download(name)]));
        }
        FileDialogKind::PickFolder | FileDialogKind::PickFolders => {
            reply.send(DialogResult::Failed(FileError::Unsupported(
                "a browser cannot choose a folder".into(),
            )));
        }
        kind => {
            let builder = configure(dialog);
            wasm_bindgen_futures::spawn_local(async move {
                let handles = if kind == FileDialogKind::OpenFiles {
                    builder.pick_files().await
                } else {
                    builder.pick_file().await.map(|h| vec![h])
                };
                reply.send(match handles {
                    Some(handles) if !handles.is_empty() => DialogResult::Picked(
                        handles
                            .iter()
                            .map(|h| PickedFile::from_web(h.inner().clone()))
                            .collect(),
                    ),
                    _ => DialogResult::Cancelled,
                });
            });
        }
    }
}
