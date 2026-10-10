//! Application-owned IDE panel trees, composed from TabBar, SplitPane and Window.
mod access;
mod anim;
mod drag;
mod drop_zones;
mod float;
mod focus_ring;
mod input;
mod layout;
mod model;
mod normalize;
mod operations;
mod options;
mod paint;
mod persist;
mod show;
mod style;

pub(crate) use anim::DockRuntime;
pub use drop_zones::{DockDropZone, DockSide};
pub use model::{DockChild, DockError, DockFloat, DockIssue, DockNode, DockState, PanelId};
pub use options::{Dock, DockActions, DockEvent, DockOutput, DockPanelOutput, DockViewer};
pub use style::{DockMotion, DockPreset, DockStyle};
