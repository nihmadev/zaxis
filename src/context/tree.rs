//! Tree input uses the normal pointer capture and focus engine.
use super::{interaction::ClickSequence, Context, HitAction, HitRegion};
use crate::components::tree_view::TreeInput;
use winit::{event::ElementState, keyboard::KeyCode};

impl Context {
    pub(super) fn tree_key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool {
        let Some(id) = self.focused_widget.filter(|id| {
            self.previous_hits
                .iter()
                .any(|h| h.id == *id && h.action == HitAction::Tree)
        }) else {
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
        if state == ElementState::Pressed {
            self.input.keys_down.insert(code);
            self.input.keys_pressed.insert(code);
            if !repeat || !matches!(code, KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter) {
                self.tree_input
                    .entry(id)
                    .or_default()
                    .push(TreeInput::Key(code));
            }
        } else {
            self.input.keys_down.remove(&code);
            self.input.keys_released.insert(code);
        }
        true
    }
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
        let double = self.tree_click.as_ref().is_some_and(|last| {
            last.id == hit.id
                && last.count == 1
                && now.saturating_duration_since(last.time) <= std::time::Duration::from_millis(500)
                && (pointer - last.position).length_squared() <= 16.0
        });
        self.tree_click = Some(ClickSequence {
            id: hit.id,
            position: pointer,
            time: now,
            count: if double { 2 } else { 1 },
        });
        self.tree_input
            .entry(tree)
            .or_default()
            .push(TreeInput::Click {
                node,
                chevron,
                double,
            });
    }
    pub(super) fn tree_context(&mut self, pointer: crate::Vec2) -> bool {
        if let Some(hit) = self.hit_test(pointer) {
            if let HitAction::TreeRow { tree, node, .. } = hit.action {
                self.tree_input
                    .entry(tree)
                    .or_default()
                    .push(TreeInput::Context(node));
                return true;
            }
        }
        false
    }
    pub(super) fn tree_tab_action(&self, id: crate::Id) -> bool {
        self.trees
            .values()
            .any(|state| state.action_ids.contains_key(&id))
    }
    pub(super) fn tree_tab_owner(&self, id: crate::Id) -> crate::Id {
        self.trees
            .iter()
            .find(|(_, state)| state.action_ids.contains_key(&id))
            .map_or(id, |(id, _)| *id)
    }
}
