use super::{action_id, Action, Binding, Keymap};
use crate::{context::invalid_value, Id};
use std::{collections::HashMap, hash::Hash};

/// The actions of an application and their keymap: declared once, shown and run from menus,
/// buttons and the keyboard.
///
/// Build it at startup and give it to the context with
/// [`Context::set_actions`](crate::Context::set_actions). A bad declaration never panics:
/// an empty list, a duplicate id, an empty title or an unreadable shortcut is reported as an
/// [`InvalidValue`](crate::DiagnosticKind::InvalidValue) diagnostic and the nearest valid
/// thing is used.
///
/// ```
/// use zaxis::{Action, Actions, Mods};
/// use zaxis::winit::keyboard::KeyCode;
/// let actions = Actions::new()
///     .register(Action::new("file.save", "Save").shortcut(Mods::PRIMARY.key(KeyCode::KeyS)))
///     .register(Action::new("file.quit", "Quit").shortcut(Mods::PRIMARY.key(KeyCode::KeyQ)));
/// assert_eq!(actions.len(), 2);
/// assert!(actions.keymap().conflicts().is_empty());
/// ```
#[derive(Clone, Debug, Default)]
pub struct Actions {
    list: Vec<Action>,
    index: HashMap<Id, usize>,
    keymap: Keymap,
}

impl Actions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an action. A second declaration of an id is ignored. Shortcuts that collide with
    /// earlier ones are reported; see [`Keymap::conflicts`].
    #[track_caller]
    pub fn register(mut self, action: Action) -> Self {
        self.insert(action);
        self
    }

    /// Like [`register`](Self::register) on a registry you already hold. Returns whether
    /// the action was added.
    #[track_caller]
    pub fn insert(&mut self, mut action: Action) -> bool {
        if self.index.contains_key(&action.id) {
            invalid_value(
                "Actions::register",
                format!(
                    "duplicate action id for \"{}\"; the first is kept",
                    action.title
                ),
            );
            return false;
        }
        if action.title.trim().is_empty() {
            invalid_value(
                "Actions::register",
                format!("empty title for action `{}`; using its name", action.name),
            );
            action.title = action.name.clone();
        }
        self.keymap.add_action(action.id, &action.name);
        let mut clashes = Vec::new();
        for (context, chord) in &action.shortcuts {
            clashes.extend(self.keymap.add_default(Binding {
                action: action.id,
                context: context.clone(),
                chord: chord.clone(),
            }));
        }
        if !clashes.is_empty() {
            let named = |id: Id| self.keymap.name_of(id).unwrap_or("?").to_owned();
            invalid_value(
                "Actions::register",
                format!(
                    "shortcut of \"{}\" collides with `{}`; both stay, the first wins",
                    action.title,
                    named(clashes[0].first)
                ),
            );
        }
        self.index.insert(action.id, self.list.len());
        self.list.push(action);
        true
    }

    /// The action declared with this value.
    pub fn get(&self, id: impl Hash) -> Option<&Action> {
        self.action(action_id(id))
    }

    pub(crate) fn action(&self, id: Id) -> Option<&Action> {
        self.index.get(&id).map(|index| &self.list[*index])
    }

    /// Actions in the order they were declared.
    pub fn iter(&self) -> impl Iterator<Item = &Action> {
        self.list.iter()
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }
}
