//! Tree views: keys of the focused tree, row clicks (double clicks included) and context
//! clicks routed through the normal pointer capture and focus engine, and the retained
//! state of every tree.
use super::{gesture::ClickCounter, Context, HitAction, HitRegion, Id};
use crate::components::tree_view::{TreeInput, TreeState};
use std::collections::HashMap;
use winit::{event::ElementState, keyboard::KeyCode};

#[derive(Default)]
pub(crate) struct Trees {
    pub(crate) states: HashMap<Id, TreeState>,
    /// Input waiting for each tree's next pass.
    input: HashMap<Id, Vec<TreeInput>>,
    /// Row clicks: a second click on the same row is a double click.
    clicks: ClickCounter,
}

impl Trees {
    /// Trees not built in pass `frame` lose their state.
    pub(super) fn retire(&mut self, frame: u64) {
        self.states.retain(|_, state| state.last_frame == frame);
    }

    pub(super) fn clear_input(&mut self) {
        self.input.clear();
    }

    /// Whether `id` is an action button inside a tree row: Tab skips those.
    pub(super) fn is_row_action(&self, id: Id) -> bool {
        self.states
            .values()
            .any(|state| state.action_ids.contains_key(&id))
    }

    /// The tree that owns row action `id`, or `id` itself.
    pub(super) fn tab_owner(&self, id: Id) -> Id {
        self.states
            .iter()
            .find(|(_, state)| state.action_ids.contains_key(&id))
            .map_or(id, |(id, _)| *id)
    }

    pub(super) fn pending(&self) -> usize {
        self.input.len()
    }
}

impl Context {
    pub(super) fn tree_key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool {
        let Some(id) = self.interaction.focused_as(HitAction::Tree) else {
            return false;
        };
        if !matches!(
            code,
            KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Space
        ) {
            return false;
        }
        let pressed = state == ElementState::Pressed;
        self.input.record_key(code, pressed);
        if pressed
            && (!repeat || !matches!(code, KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter))
        {
            self.push_tree_input(id, TreeInput::Key(code));
        }
        true
    }

    /// A release over the row that took the press: a click, or the second of a double click.
    pub(super) fn tree_click(&mut self, hit: HitRegion) {
        let HitAction::TreeRow {
            tree,
            node,
            chevron,
        } = hit.action
        else {
            return;
        };
        let pointer = self.input.pointer.unwrap_or(hit.rect.center());
        let now = crate::time::Instant::now();
        let double = self.trees.clicks.count(hit.id, pointer, now, 2, true) == 2;
        self.push_tree_input(
            tree,
            TreeInput::Click {
                node,
                chevron,
                double,
            },
        );
    }

    /// A secondary press on a row. Returns whether one was under the pointer.
    pub(super) fn tree_context(&mut self, pointer: crate::Vec2) -> bool {
        if let Some(hit) = self.hit_test(pointer) {
            if let HitAction::TreeRow { tree, node, .. } = hit.action {
                self.push_tree_input(tree, TreeInput::Context(node));
                return true;
            }
        }
        false
    }

    fn push_tree_input(&mut self, tree: Id, input: TreeInput) {
        self.trees.input.entry(tree).or_default().push(input);
    }

    /// Input of tree `id` for this pass.
    pub(crate) fn take_tree_input(&mut self, id: Id) -> Vec<TreeInput> {
        self.trees.input.remove(&id).unwrap_or_default()
    }
}
