//! The commands of the notes window, declared once. Menus, the context menu, the toolbar
//! and the keyboard all run these.

use zaxis::winit::keyboard::KeyCode;
use zaxis::{Action, Actions, Mods};

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Cmd {
    New,
    Save,
    Revert,
    Quit,
    Clear,
    Wrap,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    Statistics,
    Shortcuts,
}

impl Cmd {
    pub const ALL: [Self; 11] = [
        Self::New,
        Self::Save,
        Self::Revert,
        Self::Quit,
        Self::Clear,
        Self::Wrap,
        Self::ZoomIn,
        Self::ZoomOut,
        Self::ZoomReset,
        Self::Statistics,
        Self::Shortcuts,
    ];
}

pub fn registry() -> Actions {
    let primary = |key| Mods::PRIMARY.key(key);
    Actions::new()
        .register(
            Action::new(Cmd::New, "New")
                .group("File")
                .shortcut(primary(KeyCode::KeyN)),
        )
        .register(
            Action::new(Cmd::Save, "Save")
                .group("File")
                .shortcut(primary(KeyCode::KeyS)),
        )
        .register(
            Action::new(Cmd::Revert, "Revert")
                .group("File")
                .shortcut((Mods::PRIMARY | Mods::SHIFT).key(KeyCode::KeyR)),
        )
        .register(
            Action::new(Cmd::Quit, "Quit")
                .group("File")
                .shortcut(primary(KeyCode::KeyQ)),
        )
        .register(
            Action::new(Cmd::Clear, "Clear")
                .group("Edit")
                .description("Remove all the text")
                .shortcut_in(
                    "editor",
                    primary(KeyCode::KeyK).then(primary(KeyCode::KeyC)),
                ),
        )
        .register(
            Action::new(Cmd::Wrap, "Word wrap")
                .group("View")
                .shortcut(Mods::ALT.key(KeyCode::KeyZ)),
        )
        .register(
            Action::new(Cmd::ZoomIn, "Zoom in")
                .group("View")
                .shortcut(primary(KeyCode::Equal))
                .shortcut(primary(KeyCode::NumpadAdd)),
        )
        .register(
            Action::new(Cmd::ZoomOut, "Zoom out")
                .group("View")
                .shortcut(primary(KeyCode::Minus))
                .shortcut(primary(KeyCode::NumpadSubtract)),
        )
        .register(
            Action::new(Cmd::ZoomReset, "Actual size")
                .group("View")
                .shortcut(primary(KeyCode::Digit0)),
        )
        .register(
            Action::new(Cmd::Statistics, "Statistics")
                .group("View")
                .shortcut(primary(KeyCode::KeyB)),
        )
        .register(
            Action::new(Cmd::Shortcuts, "Keyboard shortcuts")
                .group("Tools")
                .shortcut(primary(KeyCode::Comma)),
        )
}
