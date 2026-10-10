//! Where focus goes: the Tab stop of a group and the step an arrow key takes.

use super::{FocusAxis, FocusGroups};
use crate::context::Id;
use std::collections::HashMap;
use winit::keyboard::KeyCode;

/// A move within a group.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Previous,
    Next,
    First,
    Last,
}

impl FocusAxis {
    fn step(self, code: KeyCode) -> Option<Step> {
        let (previous, next) = match self {
            Self::Horizontal => ([KeyCode::ArrowLeft; 2], [KeyCode::ArrowRight; 2]),
            Self::Vertical => ([KeyCode::ArrowUp; 2], [KeyCode::ArrowDown; 2]),
            Self::Both => (
                [KeyCode::ArrowLeft, KeyCode::ArrowUp],
                [KeyCode::ArrowRight, KeyCode::ArrowDown],
            ),
            Self::None => return None,
        };
        match code {
            KeyCode::Home => Some(Step::First),
            KeyCode::End => Some(Step::Last),
            code if previous.contains(&code) => Some(Step::Previous),
            code if next.contains(&code) => Some(Step::Next),
            _ => None,
        }
    }
}

/// What a navigation key does from the focused member.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Nav {
    /// No group around the focus uses this key.
    Unhandled,
    /// A group owns the key but focus cannot move: the end of the group, or one member.
    Stay,
    /// Focus moves to this region; `group` is where the move happened.
    Move { target: Id, group: Id },
}

impl FocusGroups {
    /// The region of the group `id` that Tab lands on: where focus is when it is inside,
    /// else the entry the group named, else the member focus was last on, else the first.
    /// A nested group stands for its own stop.
    pub(crate) fn stop(&self, id: Id, focused: Option<Id>) -> Option<Id> {
        let group = self.groups.get(&id)?;
        let inside = focused.is_some_and(|focus| self.contains_focus(id, Some(focus)));
        let item = (!inside)
            .then_some(group.entry)
            .flatten()
            .or_else(|| self.memory.get(&id).map(|memory| memory.item))
            .or_else(|| group.items.first().copied())?;
        self.resolve(item, focused)
    }

    /// The region a member stands for: itself, or the stop of a nested group.
    fn resolve(&self, item: Id, focused: Option<Id>) -> Option<Id> {
        if self.groups.contains_key(&item) {
            self.stop(item, focused)
        } else {
            Some(item)
        }
    }

    /// The outermost group around `id`, if it is in one.
    fn root(&self, id: Id) -> Option<Id> {
        let mut group = *self.parent.get(&id)?;
        while let Some(&outer) = self.parent.get(&group) {
            group = outer;
        }
        Some(group)
    }

    /// Whether Tab visits region `id`: every region outside groups, and the one stop of
    /// each group. `stops` caches the stops found for the groups of one traversal.
    pub(crate) fn is_tab_stop(
        &self,
        id: Id,
        focused: Option<Id>,
        stops: &mut HashMap<Id, Option<Id>>,
    ) -> bool {
        match self.root(id) {
            None => true,
            Some(root) => {
                *stops
                    .entry(root)
                    .or_insert_with(|| self.stop(root, focused))
                    == Some(id)
            }
        }
    }

    /// The region that stands for `id` in Tab order: `id`, or the stop of its group.
    pub(crate) fn tab_representative(&self, id: Id, focused: Option<Id>) -> Id {
        self.root(id)
            .and_then(|root| self.stop(root, focused))
            .unwrap_or(id)
    }

    /// Where `code` moves focus from the member `focused`. The innermost group whose axis
    /// uses the key moves it; at the end of a group without wrap an arrow key goes on to
    /// the group around, where the whole nested group is one member, so one key press
    /// never moves two levels. Home and End stay with the innermost group.
    pub(crate) fn navigate(&self, focused: Id, code: KeyCode) -> Nav {
        let mut item = focused;
        let mut handled = false;
        while let Some(&id) = self.parent.get(&item) {
            let group = &self.groups[&id];
            if group.axis == FocusAxis::None {
                // A slot navigates itself: its members' own keys decide, and the group
                // around it must not also act on them.
                return Nav::Unhandled;
            }
            let Some(step) = group.axis.step(code) else {
                item = id;
                continue;
            };
            handled = true;
            let Some(at) = group.items.iter().position(|member| *member == item) else {
                return Nav::Unhandled;
            };
            let last = group.items.len() - 1;
            let to = match step {
                Step::First => (at != 0).then_some(0),
                Step::Last => (at != last).then_some(last),
                Step::Previous if at > 0 => Some(at - 1),
                Step::Next if at < last => Some(at + 1),
                Step::Previous | Step::Next if group.wrap && last > 0 => {
                    Some(if step == Step::Next { 0 } else { last })
                }
                Step::Previous | Step::Next => None,
            };
            match (to, step) {
                (Some(to), _) => {
                    return match self.resolve(group.items[to], Some(focused)) {
                        Some(target) => Nav::Move { target, group: id },
                        None => Nav::Stay,
                    };
                }
                (None, Step::First | Step::Last) => return Nav::Stay,
                (None, _) => item = id,
            }
        }
        if handled {
            Nav::Stay
        } else {
            Nav::Unhandled
        }
    }
}
