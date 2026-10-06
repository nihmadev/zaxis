//! Showing the dialogs a window's frame opened.

use super::Runner;
use crate::{
    app::{App, WindowKey},
    files::DialogLaunch,
};

impl<A: App> Runner<A> {
    /// Hand the dialogs `key` opened to the backend. Each is modal for its own parent window
    /// (the opener unless the dialog names another open window) and for no other.
    pub(super) fn launch_dialogs(&mut self, key: &WindowKey, launches: Vec<DialogLaunch>) {
        for launch in launches {
            let parent = launch
                .dialog
                .parent_window()
                .filter(|parent| self.slots.contains_key(*parent))
                .unwrap_or(key)
                .clone();
            let window = self
                .slots
                .get(&parent)
                .and_then(|slot| slot.native.as_ref())
                .map(|native| std::sync::Arc::clone(&native.window));
            self.dialogs.launch(key, launch, &parent, window.as_ref());
        }
    }
}
