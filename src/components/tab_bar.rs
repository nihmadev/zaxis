//! `TabBar`: tabs on a strip, with icons, close buttons, reordering and drop areas.
//! `Ui::tab_bar` is its shorthand.

mod access;
mod bar;
mod drag;
mod input;
mod item;
mod layout;
mod menu;
mod options;
mod paint;
mod row;
mod show;
pub(crate) mod state;
mod style;
mod view;
mod zone;

pub use bar::TabBar;
pub use input::tab_after_close;
pub use item::TabItem;
pub use options::{
    TabActivation, TabBarOutput, TabDrag, TabMove, TabRelease, TabVariant, TabWidth,
};
pub use zone::{TabDropZone, TabDropZoneOutput};
