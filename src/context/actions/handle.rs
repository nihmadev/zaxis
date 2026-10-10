use super::super::Context;
use crate::actions::{action_id, Actions, Chord, Keymap};
use std::hash::Hash;

/// What raised an action's event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ActionSource {
    Keyboard,
    Menu,
    ContextMenu,
    Button,
    /// Code called [`ActionsHandle::trigger`].
    Programmatic,
}

/// A chord that has begun and waits for its next stroke.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingChord {
    /// The strokes pressed so far.
    pub strokes: Chord,
    /// Them as shortcut text followed by an ellipsis, for a status bar: `Ctrl+K …`.
    pub text: String,
}

/// The actions of a [`Context`]: set what is possible this pass, take what was asked for.
///
/// Reach it with `ui.actions()` or `context.actions()`. Every method takes the value the
/// action was declared with, `Action::new(File::Save, ..)` and `triggered(File::Save)`, or
/// the [`Id`](crate::Id) the action reports.
///
/// `set_enabled` and `set_checked` describe the pass being built: they are cleared at the
/// start of the next one, so a state the application stops setting returns to enabled and
/// unchecked instead of sticking. Set them before the widgets that show them. Keys that
/// arrive between two passes see what the last pass showed.
///
/// An event is raised once per user gesture, whichever widget or key it came from, and
/// [`triggered`](Self::triggered) removes it: it answers `true` once. The event waits one
/// pass for the application, so it makes no difference whether the application asks before
/// or after the widget that raised it.
pub struct ActionsHandle<'a> {
    context: &'a mut Context,
}

impl<'a> ActionsHandle<'a> {
    pub(in crate::context) fn new(context: &'a mut Context) -> Self {
        Self { context }
    }

    pub fn set_enabled(&mut self, id: impl Hash, enabled: bool) {
        let id = action_id(id);
        self.context.actions.frame.entry(id).or_default().enabled = Some(enabled);
    }

    pub fn set_checked(&mut self, id: impl Hash, checked: bool) {
        let id = action_id(id);
        self.context.actions.frame.entry(id).or_default().checked = Some(checked);
    }

    /// Whether the action can run now: what this pass set, or enabled.
    pub fn is_enabled(&self, id: impl Hash) -> bool {
        self.context.action_enabled(action_id(id))
    }

    /// The checked state set for this pass, if the application set one.
    pub fn checked(&self, id: impl Hash) -> Option<bool> {
        self.context.flags(action_id(id)).checked
    }

    /// Whether the action was asked for since the last time this answered `true`.
    /// Answers `true` once per user gesture.
    pub fn triggered(&mut self, id: impl Hash) -> bool {
        self.take(id).is_some()
    }

    /// Like [`triggered`](Self::triggered), telling what raised the event.
    pub fn take(&mut self, id: impl Hash) -> Option<ActionSource> {
        let id = action_id(id);
        let events = &mut self.context.actions.events;
        let at = events.iter().position(|event| event.id == id)?;
        Some(events.remove(at).source)
    }

    /// Ask for the action from code, along the same path as a key or a menu. Returns
    /// whether the event was raised; a disabled or unknown action raises none.
    pub fn trigger(&mut self, id: impl Hash) -> bool {
        self.context
            .fire_action(action_id(id), ActionSource::Programmatic)
    }

    /// Name the context keys are matched in from the next key on, such as `editor` or
    /// `editor/find`. Like the other state it lasts one pass: set it every pass, from where
    /// the focus is. The unscoped bindings always apply.
    pub fn set_context(&mut self, context: impl Into<String>) {
        self.context.actions.context = Some(context.into());
    }

    /// The chord waiting for its next stroke, if any.
    pub fn pending_chord(&self) -> Option<PendingChord> {
        self.context.pending_chord()
    }

    /// The shortcut text of an action as menus show it, if it has a binding.
    pub fn shortcut_text(&mut self, id: impl Hash) -> Option<String> {
        let view = self.context.action_view(action_id(id))?;
        (!view.shortcut.is_empty()).then_some(view.shortcut)
    }

    pub fn registry(&self) -> &Actions {
        &self.context.actions.registry
    }

    pub fn registry_mut(&mut self) -> &mut Actions {
        &mut self.context.actions.registry
    }

    pub fn keymap(&self) -> &Keymap {
        self.context.actions.registry.keymap()
    }

    /// Change bindings at run time, for example with [`Keymap::rebind`].
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        self.context.actions.registry.keymap_mut()
    }
}
