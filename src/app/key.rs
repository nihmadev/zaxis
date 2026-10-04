//! Application-chosen window identity.

use std::{fmt, sync::Arc};

/// A stable name for a native window, chosen by the application.
///
/// The key, not an index or an OS handle, identifies a window across closing and reopening:
/// opening a key that is already open does nothing, and reopening a closed key creates a
/// fresh window with fresh UI state. Keys are cheap to clone and compare.
///
/// ```
/// use zaxis::WindowKey;
/// let inspector = WindowKey::new("inspector");
/// assert_eq!(inspector, WindowKey::from("inspector"));
/// assert_ne!(inspector, WindowKey::main());
/// let per_document = WindowKey::new(format!("editor-{}", 3));
/// assert_eq!(per_document.as_str(), "editor-3");
/// ```
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct WindowKey(Arc<str>);

impl WindowKey {
    /// The key of the first window unless [`RunOptions::main_window`](crate::RunOptions) says otherwise.
    pub const MAIN: &'static str = "main";

    pub fn new(name: impl AsRef<str>) -> Self {
        Self(Arc::from(name.as_ref()))
    }

    /// The default key of the main window.
    pub fn main() -> Self {
        Self::new(Self::MAIN)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WindowKey {
    fn default() -> Self {
        Self::main()
    }
}

impl fmt::Debug for WindowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "WindowKey({:?})", self.0)
    }
}

impl fmt::Display for WindowKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for WindowKey {
    fn from(name: &str) -> Self {
        Self::new(name)
    }
}

impl From<String> for WindowKey {
    fn from(name: String) -> Self {
        Self::new(name)
    }
}

impl From<&WindowKey> for WindowKey {
    fn from(key: &WindowKey) -> Self {
        key.clone()
    }
}
