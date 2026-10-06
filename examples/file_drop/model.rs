//! Application data: the accepted files, the open previews and the dialogs in flight.
use std::sync::Arc;
use zaxis::{
    Context, DialogRequest, DialogResult, FileDialog, FileError, FileFilter, FileTask, ImageSource,
    PickedFile,
};

/// Text previews stop here; bigger files are only listed.
const TEXT_LIMIT: u64 = 256 * 1024;

pub enum Preview {
    None,
    Loading(usize, FileTask<Vec<u8>>),
    Image(ImageSource),
    Text,
    Folder,
    Failed(String),
}

/// One button, one dialog: the handle lives until its result is taken.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    OpenSeveral,
    SaveAs,
    ChooseFolder,
}

impl Action {
    pub const ALL: [Self; 4] = [
        Self::Open,
        Self::OpenSeveral,
        Self::SaveAs,
        Self::ChooseFolder,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "Open…",
            Self::OpenSeveral => "Open several…",
            Self::SaveAs => "Save as…",
            Self::ChooseFolder => "Choose folder…",
        }
    }

    fn dialog(self) -> FileDialog {
        match self {
            Self::Open => FileDialog::open_file().title("Open a file"),
            Self::OpenSeveral => FileDialog::open_files()
                .title("Open files")
                .filter(FileFilter::new(
                    "Images",
                    &["png", "jpg", "jpeg", "webp", "bmp"],
                ))
                .filter(FileFilter::new(
                    "Text",
                    &["txt", "md", "json", "toml", "rs"],
                )),
            Self::SaveAs => FileDialog::save_file()
                .title("Save the preview text")
                .file_name("notes.txt")
                .filter(FileFilter::new("Text", &["txt"])),
            Self::ChooseFolder => FileDialog::pick_folder().title("Choose a folder"),
        }
    }
}

pub struct Model {
    pub files: Vec<PickedFile>,
    pub selected: Option<usize>,
    pub preview: Preview,
    /// The text of the selected file, editable, and what "Save as…" writes.
    pub text: String,
    pub error: Option<String>,
    dialogs: [Option<DialogRequest>; 4],
    saving: Option<FileTask<()>>,
}

impl Model {
    pub fn new() -> Self {
        Self {
            files: Vec::new(),
            selected: None,
            preview: Preview::None,
            text: String::new(),
            error: None,
            dialogs: [None; 4],
            saving: None,
        }
    }

    pub fn open_dialog(&mut self, c: &mut Context, action: Action) {
        let slot = Action::ALL.iter().position(|a| *a == action).unwrap();
        self.dialogs[slot] = Some(c.open_dialog(action.label(), action.dialog()));
    }

    pub fn dialog_open(&self, action: Action) -> bool {
        let slot = Action::ALL.iter().position(|a| *a == action).unwrap();
        self.dialogs[slot].is_some()
    }

    /// Take what finished since the last frame: dialog answers, a read, a write.
    pub fn pump(&mut self, c: &mut Context) {
        for (slot, action) in Action::ALL.into_iter().enumerate() {
            let Some(request) = self.dialogs[slot] else {
                continue;
            };
            let Some(result) = c.take_dialog_result(&request) else {
                continue;
            };
            self.dialogs[slot] = None;
            match (action, result) {
                (Action::SaveAs, DialogResult::Picked(files)) => {
                    let bytes = self.text.clone().into_bytes();
                    self.saving = files.first().map(|file| c.write_file(file, bytes));
                }
                (_, DialogResult::Picked(files)) => self.add(c, files),
                (_, DialogResult::Failed(error)) => self.error = Some(error.to_string()),
                (_, DialogResult::Cancelled) => {}
            }
        }
        if let Some(Err(error)) = self.saving.as_ref().and_then(FileTask::take) {
            self.error = Some(format!("Could not save: {error}"));
        }
        self.poll_preview();
    }

    pub fn add(&mut self, c: &mut Context, files: Vec<PickedFile>) {
        if files.is_empty() {
            return;
        }
        let first = self.files.len();
        self.files.extend(files);
        self.select(c, first);
    }

    pub fn select(&mut self, c: &mut Context, index: usize) {
        self.selected = Some(index);
        self.error = None;
        let file = &self.files[index];
        self.preview = if file.is_dir() {
            Preview::Folder
        } else if file.size().is_some_and(|size| size > TEXT_LIMIT) && !is_image(file) {
            Preview::Failed("Too large to preview".into())
        } else {
            Preview::Loading(index, c.read_file(file))
        };
    }

    fn poll_preview(&mut self) {
        let Preview::Loading(index, task) = &self.preview else {
            return;
        };
        let Some(result) = task.take() else {
            return;
        };
        let file = &self.files[*index];
        self.preview = match result {
            Ok(bytes) if is_image(file) => Preview::Image(ImageSource::encoded(Arc::new(bytes))),
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(text) => {
                    self.text = text;
                    Preview::Text
                }
                Err(_) => Preview::Failed("Not a text file".into()),
            },
            Err(
                FileError::Io(reason)
                | FileError::Unsupported(reason)
                | FileError::Platform(reason),
            ) => Preview::Failed(reason),
        };
    }
}

pub fn is_image(file: &PickedFile) -> bool {
    file.mime()
        .is_some_and(|mime| mime.starts_with("image/") && !mime.ends_with("tiff"))
}

pub fn human_size(file: &PickedFile) -> String {
    match file.size() {
        None => String::new(),
        Some(bytes) if bytes < 1024 => format!("{bytes} B"),
        Some(bytes) if bytes < 1024 * 1024 => format!("{:.1} KB", bytes as f64 / 1024.0),
        Some(bytes) => format!("{:.1} MB", bytes as f64 / 1048576.0),
    }
}

pub fn kind(file: &PickedFile) -> String {
    if file.is_dir() {
        "Folder".to_owned()
    } else {
        file.mime()
            .map(str::to_owned)
            .or_else(|| file.extension().map(|e| e.to_ascii_uppercase()))
            .unwrap_or_else(|| "File".to_owned())
    }
}
