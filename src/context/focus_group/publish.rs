//! From the groups built in a pass to the groups that route input: reachable members only,
//! the memory of each group, and the focus events of the group itself.

use super::{Entry, FocusGroups, Group, Memory};
use crate::context::{gesture::Gestures, id_map::IdSet, HitRegion, Id};

impl FocusGroups {
    /// Publish the groups of this pass against the regions it published. A member is
    /// reachable when one of those regions takes focus: hidden, disabled, clipped away and
    /// removed members have none. An empty group is not published and loses its memory.
    pub(crate) fn publish(&mut self, hits: &[HitRegion], focused: Option<Id>) {
        let building = std::mem::take(&mut self.building);
        let registered = std::mem::take(&mut self.registered);
        self.seen.clear();
        self.open.clear();
        self.groups.clear();
        self.parent.clear();
        if building.is_empty() {
            self.memory.clear();
            return;
        }
        let present: IdSet = hits
            .iter()
            .filter(|hit| hit.action.focusable() && registered.contains_key(&hit.id))
            .map(|hit| hit.id)
            .collect();
        // A nested group is reachable when it has a reachable member: they are built after
        // the group around them, so walking backwards resolves them first.
        let mut reachable: Vec<Vec<Id>> = vec![Vec::new(); building.len()];
        for (index, group) in building.iter().enumerate().rev() {
            reachable[index] = group
                .items
                .iter()
                .filter_map(|entry| match *entry {
                    Entry::Member(id) => present.contains(&id).then_some(id),
                    Entry::Group(child) => {
                        (!reachable[child].is_empty()).then_some(building[child].id)
                    }
                })
                .collect();
        }
        for (index, open) in building.iter().enumerate() {
            let items = std::mem::take(&mut reachable[index]);
            if items.is_empty() {
                continue;
            }
            for item in &items {
                self.parent.insert(*item, open.id);
            }
            let entry = open.entry.filter(|entry| items.contains(entry));
            self.groups.insert(
                open.id,
                Group {
                    axis: open.axis,
                    wrap: open.wrap,
                    items,
                    entry,
                },
            );
        }
        self.fit_memory();
        if let Some(focus) = focused {
            self.remember(focus);
        }
    }

    /// Memory of groups that are gone is dropped. Where the remembered member is gone, the
    /// nearest reachable one in published order takes its place: the one that now stands
    /// at its position, or the last.
    fn fit_memory(&mut self) {
        let groups = &self.groups;
        self.memory.retain(|id, _| groups.contains_key(id));
        for (id, memory) in &mut self.memory {
            let items = &groups[id].items;
            match items.iter().position(|item| *item == memory.item) {
                Some(index) => memory.index = index,
                None => {
                    memory.index = memory.index.min(items.len() - 1);
                    memory.item = items[memory.index];
                }
            }
        }
    }

    /// The groups around `id` (a member or a group), innermost first.
    pub(super) fn chain(&self, id: Option<Id>) -> Vec<Id> {
        let mut chain = Vec::new();
        let mut item = id;
        while let Some(&group) = item.and_then(|item| self.parent.get(&item)) {
            chain.push(group);
            item = Some(group);
        }
        chain
    }

    /// Focus is on `id`: every group around it remembers where.
    pub(crate) fn remember(&mut self, id: Id) {
        let mut item = id;
        while let Some(&group) = self.parent.get(&item) {
            if let Some(index) = self.groups[&group].items.iter().position(|i| *i == item) {
                self.memory.insert(group, Memory { item, index });
            }
            item = group;
        }
    }

    /// Focus moved from `lost` to `gained`: the groups it left and entered hear of it like
    /// widgets do, and the groups around the new focus remember it.
    pub(crate) fn focus_changed(
        &mut self,
        lost: Option<Id>,
        gained: Option<Id>,
        gestures: &mut Gestures,
    ) {
        if self.parent.is_empty() {
            return;
        }
        let (before, after) = (self.chain(lost), self.chain(gained));
        for group in before.iter().filter(|group| !after.contains(group)) {
            gestures.focus_changed(Some(*group), None);
        }
        for group in after.iter().filter(|group| !before.contains(group)) {
            gestures.focus_changed(None, Some(*group));
        }
        if let Some(focus) = gained {
            self.remember(focus);
        }
    }

    /// Whether focus is on a member of `group` or of a group inside it.
    pub(crate) fn contains_focus(&self, group: Id, focused: Option<Id>) -> bool {
        let mut item = focused;
        while let Some(&outer) = item.and_then(|item| self.parent.get(&item)) {
            if outer == group {
                return true;
            }
            item = Some(outer);
        }
        false
    }
}
