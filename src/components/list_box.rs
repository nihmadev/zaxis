//! `ListBox`: one virtualized, keyboard-driven list on stable keys. Rows come from a
//! [`ListModel`] and are built only near the viewport (fixed or measured heights); selection
//! belongs to the application; the list reports one [`ListEvent`] per user action.

#[doc(hidden)]
pub mod heights;
mod model;
mod nav;
mod options;
mod paint;
mod row;
mod select;
mod show;
#[doc(hidden)]
pub mod state;
mod style;

pub use model::{ListEntry, ListEntryKind, ListModel};
pub use options::{ListBox, ListEvent, ListMode, ListOutput};
pub use row::ListRow;
pub(crate) use state::ListState;
pub use style::{ListBoxStyle, ListDensity};
