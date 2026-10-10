//! Actions and keymap: commands declared once, shown and run from every place that offers
//! them. See the `actions` page of the documentation.

mod action;
mod chord;
mod conflict;
mod id;
mod kbd;
mod keymap;
mod layout;
mod mods;
mod names;
mod platform;
mod registry;
mod stroke;

pub use action::Action;
pub use chord::{Chord, ChordError, MAX_STROKES};
pub use conflict::{Conflict, ConflictKind};
pub use kbd::Kbd;
pub use keymap::{Binding, Keymap, KeymapIssue};
pub use mods::Mods;
pub use platform::Platform;
pub use registry::Actions;
pub use stroke::Stroke;

pub(crate) use id::action_id;
pub(crate) use keymap::Lookup;
pub(crate) use layout::latin_letter;
pub(crate) use names::is_modifier;
pub(crate) use names::{code as key_code, name as key_name};
