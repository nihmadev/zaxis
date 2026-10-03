use super::{ComboBoxOption, Id};
use std::collections::{HashMap, HashSet};

/// Live options are still observed every pass, including in-place label/ID/enabled
/// edits. Allocation, identity validation and Unicode folding run only on changes.
#[derive(Default)]
pub(super) struct OptionsState {
    snapshot: Vec<(Id, bool, String)>,
    query: String,
    dirty: bool,
    matches: Vec<usize>,
    folded: HashMap<Id, (String, String)>,
}
impl OptionsState {
    pub(super) fn refresh<T>(&mut self, options: &[ComboBoxOption<T>]) -> bool {
        if self.snapshot.len() == options.len()
            && self
                .snapshot
                .iter()
                .zip(options)
                .all(|((id, enabled, label), option)| {
                    *id == option.id && *enabled == option.enabled && label == &option.label
                })
        {
            return false;
        }
        let mut ids = HashSet::with_capacity(options.len());
        assert!(
            options.iter().all(|o| ids.insert(o.id)),
            "duplicate ComboBox option ID"
        );
        self.folded.retain(|id, _| ids.contains(id));
        self.snapshot.truncate(options.len());
        for (index, option) in options.iter().enumerate() {
            if let Some((id, enabled, label)) = self.snapshot.get_mut(index) {
                *id = option.id;
                *enabled = option.enabled;
                label.clone_from(&option.label);
            } else {
                self.snapshot
                    .push((option.id, option.enabled, option.label.clone()));
            }
        }
        self.dirty = true;
        true
    }
    pub(super) fn matching<T>(&mut self, options: &[ComboBoxOption<T>], query: &str) -> &[usize] {
        if !self.dirty && self.query == query {
            return &self.matches;
        }
        self.matches.clear();
        let folded_query = query.to_lowercase();
        if folded_query.is_empty() {
            self.matches.extend(0..options.len());
        } else {
            for (index, option) in options.iter().enumerate() {
                let (source, folded) = self
                    .folded
                    .entry(option.id)
                    .or_insert_with(|| (option.label.clone(), option.label.to_lowercase()));
                if source != &option.label {
                    source.clone_from(&option.label);
                    *folded = option.label.to_lowercase();
                }
                if folded.contains(&folded_query) {
                    self.matches.push(index);
                }
            }
        }
        self.query.clear();
        self.query.push_str(query);
        self.dirty = false;
        &self.matches
    }
}
