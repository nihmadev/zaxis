use super::PickedFile;

/// Which files a dialog offers or a drop zone takes: extensions without the dot, compared
/// case-insensitively. An empty filter takes every file. Directories pass only a filter
/// built with [`directories`](Self::directories).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileFilter {
    pub(crate) name: String,
    pub(crate) extensions: Vec<String>,
    pub(crate) directories: bool,
}

impl FileFilter {
    /// A filter with a label for dialogs, such as "Images", and the extensions it takes.
    pub fn new<S: AsRef<str>>(name: impl Into<String>, extensions: &[S]) -> Self {
        Self {
            name: name.into(),
            extensions: extensions
                .iter()
                .map(|e| e.as_ref().trim_start_matches('.').to_ascii_lowercase())
                .collect(),
            directories: false,
        }
    }

    /// A filter that takes any file but no directory.
    pub fn any() -> Self {
        Self::default()
    }

    /// Also take directories (a drop zone that accepts folders).
    pub fn directories(mut self, accept: bool) -> Self {
        self.directories = accept;
        self
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Whether `file` passes: a directory when directories are accepted, otherwise a file
    /// with a listed extension (or any file for an empty list).
    pub fn accepts(&self, file: &PickedFile) -> bool {
        if file.is_dir() {
            return self.directories;
        }
        self.extensions.is_empty()
            || file
                .extension()
                .is_some_and(|ext| self.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    }
}
