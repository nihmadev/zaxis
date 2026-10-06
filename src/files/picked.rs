use super::{mime, FileError};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Where a file's bytes are.
#[derive(Clone, Debug)]
pub(super) enum Source {
    Path(PathBuf),
    Memory(Arc<[u8]>),
    /// Known only by what the system told while hovering: no name, no bytes.
    Placeholder,
    /// A browser `File` from a drop or an `<input type=file>`.
    #[cfg(target_arch = "wasm32")]
    Web(web_sys::File),
    /// A browser "save": writing the bytes starts a download under this name.
    #[cfg(target_arch = "wasm32")]
    #[cfg_attr(not(feature = "file-dialogs"), allow(dead_code))]
    Download,
}

/// A file chosen in a dialog or dropped on the window, on the desktop and in a browser.
///
/// Read it with [`Context::read_file`](crate::Context::read_file), which never blocks the
/// frame, or with [`read_blocking`](Self::read_blocking) for small files on the desktop.
/// Only [`name`](Self::name) is always meaningful: [`path`](Self::path) is `None` in a
/// browser, and a file listed while it is still being dragged
/// ([`Context::hovered_files`](crate::Context::hovered_files)) may have no name or size yet
/// on platforms that do not tell.
#[derive(Clone, Debug)]
pub struct PickedFile {
    name: String,
    size: Option<u64>,
    dir: bool,
    mime: Option<Cow<'static, str>>,
    pub(super) source: Source,
}

impl PickedFile {
    /// A file on disk. The system is asked once for its size and kind; a path that cannot
    /// be inspected (gone, no permission, a broken link) still makes a file, with no size.
    /// The name is the last path component, with invalid UTF-8 replaced.
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let (size, dir) = match std::fs::metadata(&path) {
            Ok(meta) => (meta.is_file().then_some(meta.len()), meta.is_dir()),
            Err(_) => (None, false),
        };
        let name = path
            .file_name()
            .map_or_else(|| path.to_string_lossy(), |n| n.to_string_lossy())
            .into_owned();
        Self::build(name, size, dir, None, Source::Path(path))
    }

    /// A file that lives in memory: for tests, generated content, or a host with its own
    /// storage. [`path`](Self::path) is `None`.
    pub fn from_memory(name: impl Into<String>, bytes: impl Into<Arc<[u8]>>) -> Self {
        let bytes = bytes.into();
        Self::build(
            name.into(),
            Some(bytes.len() as u64),
            false,
            None,
            Source::Memory(bytes),
        )
    }

    /// A directory entry with no content, for tests and hosts: `is_dir()` is true.
    pub fn directory(name: impl Into<String>) -> Self {
        Self::build(name.into(), None, true, None, Source::Placeholder)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn placeholder(mime: Option<String>) -> Self {
        let mime = mime.filter(|m| !m.is_empty()).map(Cow::Owned);
        Self::build(String::new(), None, false, mime, Source::Placeholder)
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn from_web(file: web_sys::File) -> Self {
        let mime = Some(file.type_()).filter(|m| !m.is_empty()).map(Cow::Owned);
        let size = Some(file.size() as u64);
        Self::build(file.name(), size, false, mime, Source::Web(file))
    }

    #[cfg(all(target_arch = "wasm32", feature = "file-dialogs"))]
    pub(crate) fn download(name: String) -> Self {
        Self::build(name, None, false, None, Source::Download)
    }

    fn build(
        name: String,
        size: Option<u64>,
        dir: bool,
        mime: Option<Cow<'static, str>>,
        source: Source,
    ) -> Self {
        let mut file = Self {
            name,
            size,
            dir,
            mime,
            source,
        };
        if file.mime.is_none() && !file.dir {
            file.mime = file
                .extension()
                .and_then(mime::from_extension)
                .map(Cow::Borrowed);
        }
        file
    }

    /// The file name without its directory; empty while only a hover placeholder.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The path on the desktop; always `None` in a browser and for in-memory files.
    pub fn path(&self) -> Option<&Path> {
        match &self.source {
            Source::Path(path) => Some(path),
            _ => None,
        }
    }

    /// Size in bytes when known: not for directories or files that could not be inspected.
    pub fn size(&self) -> Option<u64> {
        self.size
    }

    /// A directory. The library never lists or reads it; the application decides.
    pub fn is_dir(&self) -> bool {
        self.dir
    }

    /// The text after the last dot of the name, without it; `None` for `.hidden` names,
    /// names with no dot, and directories.
    pub fn extension(&self) -> Option<&str> {
        if self.dir {
            return None;
        }
        match self.name.rsplit_once('.') {
            Some((stem, ext)) if !stem.is_empty() && !ext.is_empty() => Some(ext),
            _ => None,
        }
    }

    /// The media type: the browser's own, or a guess from the extension for common formats.
    pub fn mime(&self) -> Option<&str> {
        self.mime.as_deref()
    }

    /// Read the whole file now, on the calling thread. Fine for small files on the
    /// desktop; use [`Context::read_file`](crate::Context::read_file) for anything the frame
    /// should not wait for. Fails in a browser, where reading is asynchronous.
    pub fn read_blocking(&self) -> Result<Vec<u8>, FileError> {
        match &self.source {
            Source::Path(path) => std::fs::read(path).map_err(|e| FileError::io(&e)),
            Source::Memory(bytes) => Ok(bytes.to_vec()),
            _ => Err(FileError::Unsupported(
                "this file cannot be read synchronously; use Context::read_file".into(),
            )),
        }
    }
}
