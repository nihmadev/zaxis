use crate::{app::WindowKey, files::FileFilter};
use std::path::{Path, PathBuf};

/// What a dialog asks the user for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileDialogKind {
    OpenFile,
    OpenFiles,
    SaveFile,
    PickFolder,
    PickFolders,
}

impl FileDialogKind {
    /// Whether the answer may hold more than one entry.
    pub fn is_multiple(self) -> bool {
        matches!(self, Self::OpenFiles | Self::PickFolders)
    }
}

/// A native dialog to open or save files or choose folders. Build one, then pass it to
/// [`Context::open_dialog`](crate::Context::open_dialog).
///
/// ```no_run
/// use zaxis::{FileDialog, FileFilter};
/// let dialog = FileDialog::open_files()
///     .title("Add images")
///     .filter(FileFilter::new("Images", &["png", "jpg"]));
/// ```
#[derive(Clone, Debug)]
pub struct FileDialog {
    kind: FileDialogKind,
    title: Option<String>,
    directory: Option<PathBuf>,
    file_name: Option<String>,
    filters: Vec<FileFilter>,
    parent: Option<WindowKey>,
}

impl FileDialog {
    fn new(kind: FileDialogKind) -> Self {
        Self {
            kind,
            title: None,
            directory: None,
            file_name: None,
            filters: Vec::new(),
            parent: None,
        }
    }

    /// Choose one existing file.
    pub fn open_file() -> Self {
        Self::new(FileDialogKind::OpenFile)
    }

    /// Choose any number of existing files.
    pub fn open_files() -> Self {
        Self::new(FileDialogKind::OpenFiles)
    }

    /// Choose where to save. In a browser this is a download: the answer names the file
    /// and [`Context::write_file`](crate::Context::write_file) starts the download.
    pub fn save_file() -> Self {
        Self::new(FileDialogKind::SaveFile)
    }

    /// Choose one folder. Not available in a browser.
    pub fn pick_folder() -> Self {
        Self::new(FileDialogKind::PickFolder)
    }

    /// Choose several folders. Not available in a browser.
    pub fn pick_folders() -> Self {
        Self::new(FileDialogKind::PickFolders)
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// The folder the dialog starts in. Ignored where the system remembers its own.
    pub fn directory(mut self, directory: impl Into<PathBuf>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// The name offered by a save dialog.
    pub fn file_name(mut self, name: impl Into<String>) -> Self {
        self.file_name = Some(name.into());
        self
    }

    /// Offer a filter (a name and its extensions). Several can be added.
    pub fn filter(mut self, filter: FileFilter) -> Self {
        self.filters.push(filter);
        self
    }

    /// The window the dialog belongs to: it is modal for that window and for none of the
    /// others, and closing the window answers [`DialogResult::Cancelled`](super::DialogResult::Cancelled).
    /// Defaults to the window that opened it.
    pub fn parent(mut self, window: impl Into<WindowKey>) -> Self {
        self.parent = Some(window.into());
        self
    }

    pub fn kind(&self) -> FileDialogKind {
        self.kind
    }

    pub fn title_text(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn start_directory(&self) -> Option<&Path> {
        self.directory.as_deref()
    }

    pub fn default_file_name(&self) -> Option<&str> {
        self.file_name.as_deref()
    }

    pub fn filters(&self) -> &[FileFilter] {
        &self.filters
    }

    pub fn parent_window(&self) -> Option<&WindowKey> {
        self.parent.as_ref()
    }
}
