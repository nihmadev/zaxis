use super::Context;
impl Context {
    pub(super) fn finish_collapsing_headers(&mut self) {
        // Keep retained nested headers while an ancestor hides them. Removing the
        // entire ancestor releases the group. No content callbacks run here.
        let mut live: Vec<_> = self
            .collapsing_headers
            .iter()
            .filter(|(_, state)| state.last_frame == self.frame)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = std::collections::HashSet::new();
        while let Some(id) = live.pop() {
            if !visited.insert(id) {
                continue;
            }
            if let Some(state) = self.collapsing_headers.get_mut(&id) {
                state.last_frame = self.frame;
                live.extend(state.descendants.iter().copied());
            }
        }
        self.collapsing_headers
            .retain(|_, state| state.last_frame == self.frame);
    }
}
