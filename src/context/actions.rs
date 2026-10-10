//! Actions at run time: the registry, the state the application sets each pass, the events
//! that menus, buttons, keys and code raise, and the chord waiting for its next key.
//!
//! One user gesture raises one event. The application takes it with `triggered`, which
//! removes it, so an action wired to a menu, a button and a key still runs once. An event
//! lives until the end of the pass after the one it was raised in, so the application sees
//! it whether it asks before or after the widget that raised it.

use super::{Context, DiagnosticKind, Id};
use crate::actions::{Actions, Chord, Kbd, Mods, Stroke};
use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};
use winit::keyboard::KeyCode;

mod dispatch;
mod handle;

pub use handle::{ActionSource, ActionsHandle, PendingChord};

#[derive(Clone, Copy, Debug, Default)]
struct Flags {
    enabled: Option<bool>,
    checked: Option<bool>,
}

struct Event {
    id: Id,
    source: ActionSource,
    queued: u64,
}

struct Pending {
    strokes: Vec<Stroke>,
    deadline: crate::time::Instant,
}

#[derive(Default)]
pub(crate) struct ActionRuntime {
    registry: Actions,
    /// Set by the application during this pass; gone at the next one.
    frame: HashMap<Id, Flags>,
    /// What the last finished pass showed. Keys that arrive between passes read this.
    published: HashMap<Id, Flags>,
    context: Option<String>,
    published_context: String,
    events: Vec<Event>,
    chord: Option<Pending>,
    /// Keys an action took: their repeats and release do not reach anything else.
    swallowed: HashSet<KeyCode>,
    /// Keys pressed since the last pass: the key they count as and the modifiers held.
    pressed: HashMap<KeyCode, (KeyCode, Mods)>,
}

/// What a widget needs to show an action.
pub(crate) struct ActionView {
    pub(crate) title: String,
    pub(crate) description: String,
    pub(crate) shortcut: String,
    pub(crate) enabled: bool,
    pub(crate) checked: Option<bool>,
    pub(crate) icon: Option<crate::ImageSource>,
}

impl ActionRuntime {
    pub(super) fn begin_pass(&mut self, now: crate::time::Instant) {
        self.frame.clear();
        self.context = None;
        if self
            .chord
            .as_ref()
            .is_some_and(|chord| chord.deadline <= now)
        {
            self.chord = None;
        }
    }

    pub(super) fn finish_frame(&mut self, frame: u64) {
        self.published = std::mem::take(&mut self.frame);
        self.published_context = self.context.clone().unwrap_or_default();
        self.events.retain(|event| event.queued >= frame);
        self.pressed.clear();
    }

    /// The key a press counts as for shortcuts, and the modifiers held when it came.
    pub(crate) fn pressed_key(&self, code: KeyCode) -> Option<(KeyCode, Mods)> {
        self.pressed.get(&code).copied()
    }

    pub(super) fn focus_lost(&mut self) {
        self.chord = None;
        self.swallowed.clear();
    }

    /// Whether an event for the key's release or repeat was taken by an action.
    pub(super) fn release_swallowed(&mut self, code: KeyCode) -> bool {
        self.swallowed.remove(&code)
    }
}

impl Context {
    /// Install the application's actions and keymap, replacing any earlier ones. An empty
    /// set is accepted and reported as a diagnostic.
    #[track_caller]
    pub fn set_actions(&mut self, actions: Actions) {
        if actions.is_empty() {
            super::invalid_value(
                "Context::set_actions",
                "no actions registered; keys, menus and buttons have nothing to run".into(),
            );
        }
        self.actions.registry = actions;
        self.actions.chord = None;
        self.request_repaint();
    }

    /// The registered actions and keymap, for code that only reads them.
    pub fn action_registry(&self) -> &Actions {
        &self.actions.registry
    }

    /// The chord waiting for its next stroke, if any.
    pub fn pending_chord(&self) -> Option<PendingChord> {
        let strokes = self.pending_strokes()?;
        let platform = self.action_platform();
        let text = format!("{} …", Kbd::new(&strokes).platform(platform));
        Some(PendingChord { strokes, text })
    }

    /// The actions, their state and their events; see [`ActionsHandle`].
    pub fn actions(&mut self) -> ActionsHandle<'_> {
        ActionsHandle::new(self)
    }

    fn flags(&self, id: Id) -> Flags {
        let source = if self.in_pass {
            &self.actions.frame
        } else {
            &self.actions.published
        };
        source.get(&id).copied().unwrap_or_default()
    }

    pub(crate) fn action_enabled(&self, id: Id) -> bool {
        self.flags(id).enabled.unwrap_or(true)
    }

    /// Raise the event of an action, unless it is unknown or disabled. Returns whether it
    /// was raised.
    pub(crate) fn fire_action(&mut self, id: Id, source: ActionSource) -> bool {
        if self.actions.registry.action(id).is_none() {
            self.unknown_action(id);
            return false;
        }
        if !self.action_enabled(id) {
            return false;
        }
        let queued = self.frame;
        self.actions.events.push(Event { id, source, queued });
        self.request_repaint();
        true
    }

    fn unknown_action(&mut self, id: Id) {
        self.report(DiagnosticKind::InvalidUsage, Some(id), None, || {
            "unknown action: none was declared with this value".into()
        });
    }

    /// The action as a widget shows it, or `None` (reported) when it is not registered.
    pub(crate) fn action_view(&mut self, id: Id) -> Option<ActionView> {
        let Some(action) = self.actions.registry.action(id) else {
            self.unknown_action(id);
            return None;
        };
        let keymap = self.actions.registry.keymap();
        let shortcut = keymap
            .shortcut_of(id, &self.actions.published_context)
            .map(|chord| Kbd::new(chord).platform(keymap.platform()).to_string())
            .unwrap_or_default();
        let flags = self.flags(id);
        Some(ActionView {
            title: action.title.clone(),
            description: action.description.clone(),
            shortcut,
            enabled: flags.enabled.unwrap_or(true),
            checked: flags.checked,
            icon: action.icon.clone(),
        })
    }

    pub(crate) fn action_platform(&self) -> crate::Platform {
        self.actions.registry.keymap().platform()
    }

    pub(crate) fn chord_timeout(&self) -> Duration {
        self.actions.registry.keymap().chord_timeout()
    }

    /// The strokes of a chord started and not finished.
    pub(crate) fn pending_strokes(&self) -> Option<Chord> {
        let pending = self.actions.chord.as_ref()?;
        Some(Chord::new(pending.strokes.iter().copied()))
    }
}
