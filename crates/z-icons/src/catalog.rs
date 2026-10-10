use crate::{generated::ALL, Icon};

/// All bundled icons, sorted by their exact Lucide names.
///
/// Available with feature `catalog`. Using the catalog retains all SVG assets.
pub fn all() -> &'static [&'static Icon] {
    ALL
}

/// Find an icon by its exact, case-sensitive Lucide name (e.g. `"arrow-left"`).
///
/// Available with feature `catalog`. Unknown names return `None`.
///
/// ```
/// # #[cfg(feature = "catalog")] {
/// assert_eq!(z_icons::get("arrow-left"), Some(&z_icons::ARROW_LEFT));
/// assert!(z_icons::get("missing-icon").is_none());
/// # }
/// ```
pub fn get(name: &str) -> Option<&'static Icon> {
    ALL.binary_search_by_key(&name, |icon| icon.name())
        .ok()
        .map(|index| ALL[index])
}
