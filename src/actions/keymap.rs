//! The keymap: which chords run which actions, in which context.

use super::{action_id, Chord, Conflict, ConflictKind, Platform, Stroke};
use crate::{context::invalid_value, Id};
use std::{collections::HashMap, hash::Hash, time::Duration};

mod lookup;
mod store;

pub(crate) use lookup::Lookup;
pub use store::KeymapIssue;

/// One chord bound to one action. A `None` context applies everywhere.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub action: Id,
    pub context: Option<String>,
    pub chord: Chord,
}

#[derive(Clone, Debug)]
struct Entry {
    binding: Binding,
    /// The chord with `PRIMARY` resolved for the platform, as pressed keys are compared.
    resolved: Vec<Stroke>,
}

/// Bindings of chords to actions.
///
/// Every action brings its default bindings; the user's overrides replace the defaults of an
/// action as a whole. A context is a path: `editor/find` is inside `editor`, and a binding in
/// the narrower context wins over one in the wider one, with the unscoped bindings last.
/// Two bindings that collide inside one context are a [`Conflict`]; both stay in the
/// keymap, the first registered wins.
#[derive(Clone, Debug)]
pub struct Keymap {
    platform: Platform,
    timeout: Duration,
    /// Actions in registration order, with the name they have in a saved keymap.
    names: Vec<(Id, String)>,
    defaults: HashMap<Id, Vec<Binding>>,
    overrides: HashMap<Id, Vec<Binding>>,
    entries: Vec<Entry>,
    /// Entries by the first stroke of their chord.
    first: HashMap<Stroke, Vec<usize>>,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            platform: Platform::current(),
            timeout: Duration::from_millis(1500),
            names: Vec::new(),
            defaults: HashMap::new(),
            overrides: HashMap::new(),
            entries: Vec::new(),
            first: HashMap::new(),
        }
    }
}

impl Keymap {
    pub fn platform(&self) -> Platform {
        self.platform
    }

    /// Resolve [`Mods::PRIMARY`](super::Mods::PRIMARY) and write shortcuts for another
    /// platform. For tests and for previews of another system's shortcuts.
    pub fn set_platform(&mut self, platform: Platform) {
        if self.platform != platform {
            self.platform = platform;
            self.rebuild();
        }
    }

    /// How long a chord waits for its next stroke; 1.5 s.
    pub fn chord_timeout(&self) -> Duration {
        self.timeout
    }

    #[track_caller]
    pub fn set_chord_timeout(&mut self, timeout: Duration) {
        if timeout.is_zero() {
            invalid_value(
                "Keymap::set_chord_timeout",
                "expected a duration > 0; ignored".into(),
            );
        } else {
            self.timeout = timeout;
        }
    }

    /// Every binding in effect, in registration order.
    pub fn bindings(&self) -> impl Iterator<Item = &Binding> {
        self.entries.iter().map(|entry| &entry.binding)
    }

    /// The bindings of one action. Pass the value the action was declared with.
    pub fn bindings_of(&self, id: impl Hash) -> Vec<&Binding> {
        let id = action_id(id);
        self.bindings()
            .filter(|binding| binding.action == id)
            .collect()
    }

    /// The chord a menu or tooltip shows for the action: its first binding that applies in
    /// `context` or wider, else its first binding.
    pub fn shortcut_of(&self, id: Id, context: &str) -> Option<&Chord> {
        let own = || self.bindings().filter(|binding| binding.action == id);
        own()
            .find(|binding| lookup::applies(binding.context.as_deref(), context))
            .or_else(|| own().next())
            .map(|binding| &binding.chord)
    }

    /// Declare an action; its bindings follow with `add_default`.
    pub(super) fn add_action(&mut self, id: Id, name: &str) {
        self.names.push((id, name.to_owned()));
    }

    /// A default binding of a registered action. Returns the conflicts it makes.
    pub(super) fn add_default(&mut self, binding: Binding) -> Vec<Conflict> {
        let id = binding.action;
        self.defaults.entry(id).or_default().push(binding.clone());
        if self.overrides.contains_key(&id) {
            return Vec::new();
        }
        let added = self.entries.len();
        self.push_entry(binding);
        self.conflicts_with(added, true)
    }

    fn push_entry(&mut self, binding: Binding) {
        let platform = self.platform;
        let resolved: Vec<Stroke> = binding
            .chord
            .strokes()
            .iter()
            .map(|stroke| stroke.concrete(platform))
            .collect();
        if let Some(first) = resolved.first() {
            self.first
                .entry(*first)
                .or_default()
                .push(self.entries.len());
            self.entries.push(Entry { binding, resolved });
        }
    }

    /// Recompute the effective bindings from defaults and overrides.
    fn rebuild(&mut self) {
        self.entries.clear();
        self.first.clear();
        let order: Vec<Id> = self.names.iter().map(|(id, _)| *id).collect();
        for id in order {
            let list = self.overrides.get(&id).or_else(|| self.defaults.get(&id));
            for binding in list.cloned().unwrap_or_default() {
                self.push_entry(binding);
            }
        }
    }

    /// Give `id` one chord of its own, in the context of its first binding, replacing every
    /// binding it had; `None` leaves it without keys. Returns the conflicts the new chord
    /// makes, which the keymap keeps all the same: show them and let the user decide.
    #[track_caller]
    pub fn rebind(&mut self, id: impl Hash, chord: Option<Chord>) -> Vec<Conflict> {
        let id = action_id(id);
        if !self.names.iter().any(|(known, _)| *known == id) {
            invalid_value("Keymap::rebind", "unknown action; ignored".into());
            return Vec::new();
        }
        let context = self
            .entries
            .iter()
            .find(|entry| entry.binding.action == id)
            .and_then(|entry| entry.binding.context.clone());
        let bindings = chord
            .filter(|chord| !chord.is_empty())
            .map(|chord| Binding {
                action: id,
                context,
                chord,
            })
            .into_iter()
            .collect();
        self.overrides.insert(id, bindings);
        self.rebuild();
        self.conflicts_of(id)
    }

    /// Give the action its default bindings back.
    pub fn reset(&mut self, id: impl Hash) {
        if self.overrides.remove(&action_id(id)).is_some() {
            self.rebuild();
        }
    }

    /// Drop every override.
    pub fn reset_all(&mut self) {
        if !self.overrides.is_empty() {
            self.overrides.clear();
            self.rebuild();
        }
    }

    /// Whether the user changed the bindings of the action.
    pub fn is_overridden(&self, id: impl Hash) -> bool {
        self.overrides.contains_key(&action_id(id))
    }

    /// Every collision between bindings.
    pub fn conflicts(&self) -> Vec<Conflict> {
        (0..self.entries.len())
            .flat_map(|index| self.conflicts_with(index, true))
            .collect()
    }

    /// The collisions that involve one action.
    pub fn conflicts_of(&self, id: Id) -> Vec<Conflict> {
        let mut found: Vec<Conflict> = Vec::new();
        for index in (0..self.entries.len()).filter(|i| self.entries[*i].binding.action == id) {
            for conflict in self.conflicts_with(index, false) {
                if !found.contains(&conflict) {
                    found.push(conflict);
                }
            }
        }
        found
    }

    /// Collisions of entry `index` with the entries before it, or with all others.
    fn conflicts_with(&self, index: usize, older_only: bool) -> Vec<Conflict> {
        let entry = &self.entries[index];
        let Some(candidates) = entry.resolved.first().and_then(|key| self.first.get(key)) else {
            return Vec::new();
        };
        let mut found = Vec::new();
        for &other in candidates
            .iter()
            .filter(|other| **other != index && (**other < index || !older_only))
        {
            let (a, b) = (&self.entries[other], entry);
            if a.binding.action == b.binding.action || a.binding.context != b.binding.context {
                continue;
            }
            let (first, second, kind) = if a.resolved == b.resolved {
                // The one registered first wins.
                if other < index {
                    (a, b, ConflictKind::Same)
                } else {
                    (b, a, ConflictKind::Same)
                }
            } else if b.resolved.starts_with(&a.resolved) {
                (a, b, ConflictKind::Prefix)
            } else if a.resolved.starts_with(&b.resolved) {
                (b, a, ConflictKind::Prefix)
            } else {
                continue;
            };
            found.push(Conflict {
                context: first.binding.context.clone(),
                chord: first.binding.chord.clone(),
                first: first.binding.action,
                second: second.binding.action,
                kind,
            });
        }
        found
    }

    /// The saved name of a registered action.
    pub fn name_of(&self, id: Id) -> Option<&str> {
        self.names
            .iter()
            .find(|(known, _)| *known == id)
            .map(|(_, name)| name.as_str())
    }
}
