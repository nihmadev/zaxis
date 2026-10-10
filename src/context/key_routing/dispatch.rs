//! Claims and ownership on the keyboard path: publishing declarations, deciding the owner
//! of a press, and following a held key to the end of its gesture.

use super::{queue::Pushed, Claim, Owner, CLAIMS_PER_WIDGET};
use crate::{
    actions::Mods,
    context::{id_map::IdSet, key_events::KeyEvent, Context, DiagnosticKind, Id, KeyInterest},
};
use winit::{
    event::ElementState,
    keyboard::{Key, KeyCode, NativeKey, PhysicalKey},
};

impl Claim {
    fn matches(&self, code: KeyCode, layout: KeyCode, mods: Mods) -> bool {
        let key = if self.by_position { code } else { layout };
        key == self.key && self.mods.is_none_or(|wanted| wanted == mods)
    }
}

impl Context {
    /// Add the keys of `interest` to the claims `id` makes in this pass.
    pub(crate) fn claim_keys(&mut self, id: Id, interest: &KeyInterest<'_>) {
        let mods = interest
            .mods
            .map(|mods| mods.concrete(self.action_platform()));
        let list = self.keys.claims_of(id);
        let mut full = false;
        for &key in interest.keys {
            let claim = Claim {
                key,
                mods,
                by_position: interest.by_position,
                repeats: interest.repeats,
                releases: interest.releases,
            };
            if list.contains(&claim) {
                continue;
            }
            if list.len() >= CLAIMS_PER_WIDGET {
                full = true;
                break;
            }
            list.push(claim);
        }
        if full {
            self.report(DiagnosticKind::InvalidUsage, Some(id), None, || {
                "too many key claims for one widget; the extra keys are ignored".into()
            });
        }
    }

    /// The events addressed to `id`, once. Code outside the top modal gets none.
    pub(crate) fn take_key_events(&mut self, id: Id) -> Vec<KeyEvent> {
        if self.keys.pending() == 0 || self.input_blocked() {
            return Vec::new();
        }
        self.keys.take(id)
    }

    /// End of pass: the claims of this pass route the next input. A widget that claims
    /// keys but has no region that takes focus is reported, for as long as it does.
    pub(crate) fn publish_key_claims(&mut self) {
        let fresh = std::mem::take(&mut self.keys.fresh);
        if fresh || !self.keys.invalid.is_empty() {
            self.check_key_claims(fresh);
        }
        let invalid: Vec<Id> = self.keys.invalid.iter().copied().collect();
        for id in invalid {
            self.report(DiagnosticKind::InvalidUsage, Some(id), None, || {
                "key claims need a region that takes focus (Sense::FOCUS) that is not a \
                 text field, slider or other control that keeps its keys"
                    .into()
            });
        }
        self.keys.publish();
    }

    /// Find the widgets among those that declared claims (`fresh`: all of them; else the
    /// ones known to be wrong) whose claims have nothing to apply to.
    fn check_key_claims(&mut self, fresh: bool) {
        let keys = &self.keys;
        let watched = |id: &Id| {
            if fresh {
                keys.declared.contains_key(id)
            } else {
                keys.invalid.contains(id)
            }
        };
        let ok: IdSet = self
            .interaction
            .previous_hits
            .iter()
            .filter(|hit| hit.action.focusable() && !hit.action.keeps_keys() && watched(&hit.id))
            .map(|hit| hit.id)
            .collect();
        let candidates: Vec<Id> = if fresh {
            keys.declared.keys().copied().collect()
        } else {
            keys.invalid.iter().copied().collect()
        };
        for id in candidates {
            if ok.contains(&id) {
                self.keys.invalid.remove(&id);
            } else {
                self.keys.invalid.insert(id);
            }
        }
    }

    /// Whether widgets in `window` may take keys now: the leaf popup and its panels, else
    /// the top modal, else any layer. A widget under an open popup or modal gets nothing
    /// and cannot bypass it.
    pub(in crate::context) fn key_layer_open(&self, window: Id) -> bool {
        if self.popups.is_active() {
            return self.popups.is_top_layer(window);
        }
        self.top_modal_id().is_none_or(|top| window == top)
    }

    /// The autorepeat or release of a key whose press an owner took. It goes to the same
    /// owner whatever has focus now, and is consumed. A press of the key starts over.
    pub(in crate::context) fn owned_key_tail(
        &mut self,
        code: KeyCode,
        layout: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> bool {
        let Some(owner) = self.keys.owners.get(&code).copied() else {
            return false;
        };
        let pressed = state == ElementState::Pressed;
        if pressed && !repeat {
            self.keys.owners.remove(&code);
            return false;
        }
        self.input.record_key(code, pressed);
        if pressed {
            match owner {
                Owner::Navigation => {
                    self.group_key(code);
                }
                Owner::Widget {
                    id,
                    repeats: true,
                    dropped: false,
                    ..
                } => {
                    let event = self.key_event(code, layout, state, repeat);
                    self.keys.push(id, event);
                }
                Owner::Widget { .. } => {}
            }
            return true;
        }
        self.keys.owners.remove(&code);
        if let Owner::Widget {
            id,
            releases: true,
            dropped: false,
            ..
        } = owner
        {
            let event = self.key_event(code, layout, state, repeat);
            self.keys.push(id, event);
        }
        true
    }

    /// A press that the focused widget claimed: it becomes the owner of the key. Runs
    /// before Actions so that an explicit claim wins over a shortcut on the same key; a
    /// chord in progress and a key being captured by a `KeyBox` come first.
    pub(in crate::context) fn claimed_key(
        &mut self,
        code: KeyCode,
        layout: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> bool {
        if state != ElementState::Pressed
            || repeat
            || !self.keys.has_claims()
            || self.key_capture_active()
            || self.pending_strokes().is_some()
        {
            return false;
        }
        let Some(id) = self.interaction.focused else {
            return false;
        };
        let mods = Mods::from_state(self.input.modifiers);
        let Some(claims) = self.keys.published.get(&id) else {
            return false;
        };
        let mut wanted: Option<(bool, bool)> = None;
        for claim in claims.iter().filter(|c| c.matches(code, layout, mods)) {
            let (repeats, releases) = wanted.unwrap_or_default();
            wanted = Some((repeats | claim.repeats, releases | claim.releases));
        }
        let Some((repeats, releases)) = wanted else {
            return false;
        };
        let Some(hit) = self
            .interaction
            .previous_hits
            .iter()
            .find(|hit| hit.id == id && hit.action.focusable())
        else {
            return false;
        };
        if hit.action.keeps_keys() || !self.key_layer_open(hit.window) {
            return false;
        }
        let event = self.key_event(code, layout, state, repeat);
        let dropped = match self.keys.push(id, event) {
            Pushed::Queued => false,
            Pushed::Dropped { first } => {
                if first {
                    self.report(DiagnosticKind::InvalidUsage, Some(id), None, || {
                        "key queue full: events are dropped until the widget takes its keys \
                         with Ui::keys in every pass"
                            .into()
                    });
                }
                true
            }
        };
        self.keys.owners.insert(
            code,
            Owner::Widget {
                id,
                repeats,
                releases,
                dropped,
            },
        );
        self.input.record_key(code, true);
        self.keys.claimed = true;
        true
    }

    /// The event for the key being dispatched, with the platform key and the modifiers
    /// as they are now. A host that feeds physical codes has no logical key.
    fn key_event(
        &self,
        code: KeyCode,
        layout: KeyCode,
        state: ElementState,
        repeat: bool,
    ) -> KeyEvent {
        let (physical, logical, text) = self.keys.raw.clone().unwrap_or((
            PhysicalKey::Code(code),
            Key::Unidentified(NativeKey::Unidentified),
            None,
        ));
        KeyEvent {
            code,
            layout,
            physical,
            logical,
            state,
            repeat,
            modifiers: self.input.modifiers,
            text,
        }
    }
}
