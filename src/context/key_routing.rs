//! Keys addressed to a widget: the declarations of the last pass, the events waiting for
//! their owner, and the keys that are held down by an owner.
//!
//! A focused widget declares [`KeyInterest`]s every pass. At the end of the pass they are
//! published together with the hit regions, and from then on they decide the owner of a key
//! the moment it arrives: `Context::on_input` queues the [`KeyEvent`] for the focused
//! widget and reports it consumed, with no UI pass in between. The widget takes its events
//! in the next pass with `Ui::take_keys`, in arrival order. Rules that make this safe:
//!
//! * Only the focused widget is asked, and only when its region is published, takes focus,
//!   is not clipped away, belongs to the layer that may take keys (the open popup, else
//!   the top modal, else any) and does not keep its keys itself (text fields, sliders).
//! * A claimed press makes its widget the owner of that key until release: autorepeats and
//!   the release go to the same widget however focus moved, and are consumed with it.
//! * Events wait for one pass. What the owner did not take is dropped at the end of it, so
//!   hidden or removed widgets leave nothing behind.
//! * The queue is bounded. When it is full, new presses and repeats are dropped, the owner
//!   is remembered as dropped so that its repeats and release are swallowed without being
//!   queued, and releases of delivered presses are always kept: a held key is never
//!   half delivered.

mod dispatch;
mod queue;

use super::{
    id_map::{IdMap, IdSet},
    key_events::KeyEvent,
    Id,
};
use crate::actions::Mods;
use std::collections::{HashMap, VecDeque};
use winit::keyboard::{KeyCode, PhysicalKey};

/// Events waiting for one widget; further presses and repeats are dropped.
pub(crate) const QUEUE_PER_WIDGET: usize = 256;
/// Events waiting for all widgets, which only differs from the above when focus moved
/// across many widgets between two passes.
pub(crate) const QUEUE_TOTAL: usize = 1024;
/// Distinct claims one widget may declare.
pub(crate) const CLAIMS_PER_WIDGET: usize = 64;
const POOL: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Claim {
    pub(crate) key: KeyCode,
    /// `None` matches any modifiers.
    pub(crate) mods: Option<Mods>,
    pub(crate) by_position: bool,
    pub(crate) repeats: bool,
    pub(crate) releases: bool,
}

/// Who owns a key that is held down.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Owner {
    /// A widget that claimed the press. `dropped`: the queue was full, so nothing of this
    /// gesture is delivered, but all of it is consumed.
    Widget {
        id: Id,
        repeats: bool,
        releases: bool,
        dropped: bool,
    },
    /// Group navigation took the press; its release is consumed.
    Navigation,
}

#[derive(Default)]
pub(crate) struct KeyRouting {
    /// Claims of this pass, and of the last one, which routes input until this ends.
    declared: IdMap<Vec<Claim>>,
    published: IdMap<Vec<Claim>>,
    /// Widgets that declared claims the last pass did not: check they can take them.
    fresh: bool,
    /// Widgets whose claims can never apply, reported while they keep declaring them.
    invalid: IdSet,
    pool: Vec<Vec<Claim>>,
    queue: IdMap<VecDeque<KeyEvent>>,
    pending: usize,
    pub(in crate::context) owners: HashMap<KeyCode, Owner>,
    /// The platform key being dispatched, for the logical key and the text it carries.
    raw: Option<(
        PhysicalKey,
        winit::keyboard::Key,
        Option<winit::keyboard::SmolStr>,
    )>,
    /// The key being dispatched went to a widget: its text is not typed.
    claimed: bool,
    overflowed: bool,
    dropped: u64,
}

impl KeyRouting {
    pub(crate) fn has_claims(&self) -> bool {
        !self.published.is_empty()
    }

    /// The declarations of `id` in this pass, to add claims to.
    pub(crate) fn claims_of(&mut self, id: Id) -> &mut Vec<Claim> {
        let (published, pool, fresh) = (&self.published, &mut self.pool, &mut self.fresh);
        self.declared.entry(id).or_insert_with(|| {
            if !published.contains_key(&id) {
                *fresh = true;
            }
            pool.pop().unwrap_or_default()
        })
    }

    /// The claims of this pass route the next input; the lists of the pass before go back
    /// to the pool.
    pub(crate) fn publish(&mut self) {
        std::mem::swap(&mut self.published, &mut self.declared);
        for (_, mut list) in self.declared.drain() {
            list.clear();
            if self.pool.len() < POOL {
                self.pool.push(list);
            }
        }
    }

    /// End of pass: events nobody took are dropped, and the overflow notice may repeat.
    pub(crate) fn finish_frame(&mut self) {
        self.queue.clear();
        self.pending = 0;
        self.overflowed = false;
        self.invalid.retain(|id| self.published.contains_key(id));
    }

    /// The window lost focus, or a modal or popup took over: no key is held any more and
    /// nothing is waiting.
    pub(crate) fn reset(&mut self) {
        self.queue.clear();
        self.pending = 0;
        self.owners.clear();
        self.overflowed = false;
        self.claimed = false;
        self.raw = None;
    }

    pub(crate) fn set_raw(
        &mut self,
        physical: PhysicalKey,
        logical: &winit::keyboard::Key,
        text: &Option<winit::keyboard::SmolStr>,
    ) {
        if self.has_claims() {
            self.raw = Some((physical, logical.clone(), text.clone()));
        }
    }

    /// Whether the key just dispatched went to a widget, and forget the platform key.
    pub(crate) fn finish_key(&mut self) -> bool {
        self.raw = None;
        std::mem::take(&mut self.claimed)
    }

    pub(crate) fn declared(&self) -> usize {
        self.published.len()
    }
    pub(crate) fn pending(&self) -> usize {
        self.pending
    }
    pub(crate) fn owned(&self) -> usize {
        self.owners.len()
    }
    pub(crate) fn dropped(&self) -> u64 {
        self.dropped
    }
}
