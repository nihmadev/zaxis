use std::{error::Error, fmt};

/// Why a file could not be read, written or chosen. Carried as a result, never a panic.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileError {
    /// The platform or this kind of file cannot do it: a browser file has no path, a
    /// directory has no bytes, a browser cannot choose a folder.
    Unsupported(String),
    /// The file system refused: missing, unreadable, a directory, a full disk.
    Io(String),
    /// The dialog service failed or is missing, for example no `xdg-desktop-portal`.
    Platform(String),
}

impl FileError {
    pub(crate) fn io(error: &std::io::Error) -> Self {
        Self::Io(error.to_string())
    }
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(reason) | Self::Io(reason) | Self::Platform(reason) => {
                f.write_str(reason)
            }
        }
    }
}

impl Error for FileError {}
