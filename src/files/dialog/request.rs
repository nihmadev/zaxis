use crate::{files::PickedFile, Id};

/// Handle of a dialog that was opened. Cheap to copy; keep it until the result arrives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DialogRequest {
    pub(crate) id: Id,
}

impl DialogRequest {
    /// The id derived from the key given to `open_dialog`.
    pub fn id(&self) -> Id {
        self.id
    }
}

/// How a dialog ended.
#[derive(Clone, Debug)]
pub enum DialogResult {
    /// The user chose: one entry for single dialogs, at least one for multiple ones.
    Picked(Vec<PickedFile>),
    /// The user dismissed it, or its window closed first.
    Cancelled,
    /// It could not be shown: no portal, a browser refusing, a folder dialog in a browser.
    Failed(crate::files::FileError),
}
