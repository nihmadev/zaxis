use super::action_id;
use super::Chord;
use crate::{context::invalid_value, Id, ImageSource};
use std::{fmt::Debug, hash::Hash};

/// A command the application offers: what it is called, which keys run it, and where it
/// belongs. Menus, buttons and the keyboard show and run the same action; the application
/// declares it once in [`Actions`](super::Actions).
///
/// The id comes from a value, an enum variant or a `&'static str`, never from a position or
/// a label, so it survives reordering and translation. Use the same value everywhere the
/// action is named (`MenuItem::action`, `Button::action`, `triggered`); the [`Id`](crate::Id) the
/// action reports works there as well.
///
/// ```
/// use zaxis::{Action, Mods};
/// use zaxis::winit::keyboard::KeyCode;
/// #[derive(Hash, Debug)]
/// enum File { Save }
/// let save = Action::new(File::Save, "Save")
///     .shortcut(Mods::PRIMARY.key(KeyCode::KeyS))
///     .group("File")
///     .description("Write the document to disk");
/// ```
#[derive(Clone, Debug)]
pub struct Action {
    pub(crate) id: Id,
    pub(crate) name: String,
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) group: String,
    pub(crate) icon: Option<ImageSource>,
    pub(crate) shortcuts: Vec<(Option<String>, Chord)>,
    pub(crate) modal: bool,
}

impl Action {
    /// An action identified by `id`. The title is what menus and tooltips show.
    pub fn new(id: impl Hash + Debug, title: impl Into<String>) -> Self {
        let name = saved_name(&format!("{id:?}"));
        Self {
            id: action_id(&id),
            name,
            title: title.into(),
            description: String::new(),
            group: String::new(),
            icon: None,
            shortcuts: Vec::new(),
            modal: false,
        }
    }

    /// The name this action has in a saved keymap. By default the `Debug` text of the id
    /// (`"file.save"` or `Save`); set it when that text could change or collide.
    pub fn name(mut self, name: impl AsRef<str>) -> Self {
        self.name = saved_name(name.as_ref());
        self
    }

    /// A default key binding in every context. Call it again for more bindings.
    #[track_caller]
    pub fn shortcut(self, chord: impl Into<Chord>) -> Self {
        self.bind(None, chord.into())
    }

    /// A default key binding that applies only while `context` is active. Contexts nest
    /// with `/`: `editor/find` is inside `editor`.
    #[track_caller]
    pub fn shortcut_in(self, context: impl Into<String>, chord: impl Into<Chord>) -> Self {
        self.bind(Some(context.into()), chord.into())
    }

    #[track_caller]
    fn bind(mut self, context: Option<String>, chord: Chord) -> Self {
        if chord.is_empty() {
            invalid_value(
                "Action::shortcut",
                format!("expected at least one key for \"{}\"; ignored", self.title),
            );
        } else {
            self.shortcuts.push((context, chord));
        }
        self
    }

    /// Which part of the application the action belongs to, for grouping in a list.
    pub fn group(mut self, group: impl Into<String>) -> Self {
        self.group = group.into();
        self
    }

    /// One sentence for tooltips and assistive technology.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// The icon buttons show next to the title.
    pub fn icon(mut self, icon: impl Into<ImageSource>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Let the action run while a modal is open. By default a modal blocks every action.
    pub fn allowed_in_modal(mut self, allowed: bool) -> Self {
        self.modal = allowed;
        self
    }

    pub fn id(&self) -> Id {
        self.id
    }
    pub fn title(&self) -> &str {
        &self.title
    }
    pub fn saved_name(&self) -> &str {
        &self.name
    }
    pub fn description_text(&self) -> &str {
        &self.description
    }
    pub fn group_name(&self) -> &str {
        &self.group
    }
    pub fn icon_source(&self) -> Option<&ImageSource> {
        self.icon.as_ref()
    }
    pub fn is_allowed_in_modal(&self) -> bool {
        self.modal
    }
}

/// A name usable in the saved format: no whitespace, `=`, `@` or `#`, no quotes around it.
fn saved_name(text: &str) -> String {
    text.trim_matches('"')
        .chars()
        .map(|c| {
            if c.is_whitespace() || matches!(c, '=' | '@' | '#' | '"') {
                '_'
            } else {
                c
            }
        })
        .collect()
}
