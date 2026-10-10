//! Focus groups: several focusable regions that share one Tab stop and move focus among
//! themselves with the arrow keys.
//!
//! A group is a scope in the UI build ([`Ui::focus_scope`](crate::Ui) and the public
//! `FocusGroup`). Every region that takes focus and is registered inside it, in the group's
//! layer, is a member; a group built inside another one is a single member of the outer one.
//! At the end of the pass the members that are still reachable are published with the hit
//! regions, and from then on they answer three questions without a new pass:
//!
//! * which member is the group's Tab stop: where focus is when it is inside, else the
//!   entry the group named, else the member focus was last on, else the first;
//! * where an arrow key, Home or End moves focus from a member, skipping what is disabled,
//!   hidden, clipped away or gone;
//! * whether the focus is inside, which gives the group its own focus events.
//!
//! Nothing here holds a value, a callback or a closure: the group only remembers which
//! member was last focused, for the groups that were built in the last pass.

mod dispatch;
mod nav;
mod publish;

use super::{
    id_map::{IdMap, IdSet},
    HitAction, HitRegion, Id,
};
use crate::Rect;

/// Which arrow keys move focus within a group. Home and End jump to the first and last
/// member for every axis but [`FocusAxis::None`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusAxis {
    /// Left and Right.
    #[default]
    Horizontal,
    /// Up and Down.
    Vertical,
    /// Left and Up go back, Right and Down go forward, in the order of the members.
    Both,
    /// The group is one Tab stop and moves no focus: its members handle the arrow keys
    /// themselves, and no group around it acts on them either, so a key is never
    /// handled twice. Its parts are reached by a click, by their own keys or by
    /// `Context::request_focus`. Nested in another group it is the single logical member
    /// a complex part (a tab strip, a list, a stepper) makes of itself: the outer arrows
    /// move onto it and around it, Tab leaves it.
    None,
}

/// How a group ended.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GroupEnd {
    pub(crate) bounds: Option<Rect>,
    pub(crate) has_focus: bool,
}

pub(super) enum Entry {
    Member(Id),
    /// A group built inside this one: index into `FocusGroups::building`.
    Group(usize),
}

/// A group being built in this pass.
pub(super) struct Open {
    id: Id,
    window: Id,
    parent: Option<usize>,
    axis: FocusAxis,
    wrap: bool,
    items: Vec<Entry>,
    entry: Option<Id>,
    bounds: Option<Rect>,
}

/// A group as the last pass published it: reachable members only, in build order.
pub(super) struct Group {
    axis: FocusAxis,
    wrap: bool,
    items: Vec<Id>,
    entry: Option<Id>,
}

/// The member of a group that focus was last on, and its place among the members.
pub(super) struct Memory {
    item: Id,
    index: usize,
}

#[derive(Default)]
pub(crate) struct FocusGroups {
    building: Vec<Open>,
    /// Indexes into `building` of the groups being built, innermost last.
    open: Vec<usize>,
    /// Members registered in this pass and the group each belongs to.
    registered: IdMap<usize>,
    /// Groups opened in this pass, to find two with one id.
    seen: IdSet,
    groups: IdMap<Group>,
    /// The group that holds each member or nested group.
    parent: IdMap<Id>,
    memory: IdMap<Memory>,
    /// The member group navigation moved focus to since the owner last looked, by group.
    moved: IdMap<Id>,
}

impl FocusGroups {
    /// Open a group. A group built in the layer of an open one belongs to it. Returns
    /// whether another group with this id was opened in the same pass.
    pub(crate) fn begin(&mut self, id: Id, window: Id, axis: FocusAxis, wrap: bool) -> bool {
        let parent = self
            .open
            .last()
            .copied()
            .filter(|&outer| self.building[outer].window == window);
        let index = self.building.len();
        if let Some(outer) = parent {
            self.building[outer].items.push(Entry::Group(index));
        }
        self.building.push(Open {
            id,
            window,
            parent,
            axis,
            wrap,
            items: Vec::new(),
            entry: None,
            bounds: None,
        });
        self.open.push(index);
        !self.seen.insert(id)
    }

    /// Close the innermost group. `bounds` are those of its members as laid out;
    /// `has_focus` says whether `focused` is one of the members registered in this pass,
    /// in the group or in one nested in it, so it holds even for a member that the last
    /// pass did not publish yet.
    pub(crate) fn end(&mut self, focused: Option<Id>) -> GroupEnd {
        let Some(index) = self.open.pop() else {
            return GroupEnd::default();
        };
        let bounds = self.building[index].bounds;
        if let (Some(outer), Some(rect)) = (self.building[index].parent, bounds) {
            self.building[outer].grow(rect);
        }
        let has_focus = focused
            .and_then(|focus| self.registered.get(&focus).copied())
            .is_some_and(|mut at| loop {
                if at == index {
                    break true;
                }
                match self.building[at].parent {
                    Some(outer) => at = outer,
                    None => break false,
                }
            });
        GroupEnd { bounds, has_focus }
    }

    /// A region was registered: it joins the innermost open group when it takes focus and
    /// lies in that group's layer. Popups and windows built inside the scope do not.
    pub(crate) fn note_hit(&mut self, hit: &HitRegion) {
        let Some(&top) = self.open.last() else {
            return;
        };
        let group = &mut self.building[top];
        if hit.window != group.window
            || !hit.action.focusable()
            || matches!(hit.action, HitAction::TreeRow { .. })
        {
            return;
        }
        if self.registered.insert(hit.id, top).is_none() {
            group.items.push(Entry::Member(hit.id));
            group.grow(hit.rect);
        }
    }

    /// The member Tab enters the innermost open group at while focus is outside it.
    pub(crate) fn set_entry(&mut self, id: Id) -> bool {
        match self.open.last() {
            Some(&top) => {
                self.building[top].entry = Some(id);
                true
            }
            None => false,
        }
    }

    pub(crate) fn is_open(&self) -> bool {
        !self.open.is_empty()
    }

    /// The member navigation moved focus to in `group` since the owner last asked.
    pub(crate) fn take_moved(&mut self, group: Id) -> Option<Id> {
        self.moved.remove(&group)
    }

    pub(crate) fn finish_frame(&mut self) {
        self.moved.clear();
    }

    pub(crate) fn len(&self) -> usize {
        self.groups.len()
    }

    pub(crate) fn members(&self) -> usize {
        self.parent.len()
    }
}

impl Open {
    fn grow(&mut self, rect: Rect) {
        self.bounds = Some(self.bounds.map_or(rect, |bounds| {
            Rect::from_min_max(bounds.min.min(rect.min), bounds.max.max(rect.max))
        }));
    }
}
