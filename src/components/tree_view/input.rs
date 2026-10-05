use super::{TreeEvent, TreeModel, TreeState};
use crate::{winit::keyboard::KeyCode, Id};

#[derive(Clone, Copy)]
pub(crate) enum TreeInput {
    Key(KeyCode),
    Click {
        node: Id,
        chevron: bool,
        double: bool,
    },
    Context(Id),
    /// Open or close a branch; does nothing when it already is. From assistive technology.
    SetOpen {
        node: Id,
        open: bool,
    },
}
impl TreeState {
    pub(super) fn input(
        &mut self,
        input: TreeInput,
        model: &impl TreeModel,
        follows: bool,
        double_expand: bool,
        events: &mut Vec<TreeEvent>,
    ) {
        match input {
            TreeInput::Context(id) => {
                if model.node(id).is_some_and(|n| n.enabled) {
                    events.push(TreeEvent::ContextAction { node: id });
                }
            }
            TreeInput::SetOpen { node, open } => {
                if model.node(node).is_some_and(|n| n.enabled) {
                    self.set_open(node, open, model, events);
                }
            }
            TreeInput::Click {
                node,
                chevron,
                double,
            } => {
                let Some(n) = model.node(node).filter(|n| n.enabled) else {
                    return;
                };
                self.focused = Some(node);
                if chevron {
                    self.set_open(node, !self.open.contains(&node), model, events);
                } else {
                    self.select(Some(node), events);
                    if double {
                        if n.children.expandable() && double_expand {
                            self.set_open(node, !self.open.contains(&node), model, events);
                        } else {
                            events.push(TreeEvent::Activated { node });
                        }
                    }
                }
            }
            TreeInput::Key(key) => {
                let Some(id) = self.focused else {
                    return;
                };
                let Some(&index) = self.index.get(&id) else {
                    return;
                };
                let row = self.rows[index];
                let position = self.accessible.binary_search(&index).unwrap_or(0);
                let mut next = Some(index);
                match key {
                    KeyCode::ArrowUp => next = position.checked_sub(1).map(|p| self.accessible[p]),
                    KeyCode::ArrowDown => next = self.accessible.get(position + 1).copied(),
                    KeyCode::Home => next = self.accessible.first().copied(),
                    KeyCode::End => next = self.accessible.last().copied(),
                    KeyCode::ArrowLeft => {
                        if row.children.expandable() && self.open.contains(&id) {
                            self.set_open(id, false, model, events);
                        } else {
                            let mut parent = row.parent;
                            while let Some(id) = parent {
                                let Some(&i) = self.index.get(&id) else {
                                    break;
                                };
                                if self.rows[i].enabled {
                                    next = Some(i);
                                    break;
                                }
                                parent = self.rows[i].parent;
                            }
                        }
                    }
                    KeyCode::ArrowRight => {
                        if row.children.expandable() && !self.open.contains(&id) {
                            self.set_open(id, true, model, events);
                        } else {
                            next = self
                                .accessible
                                .get(position + 1)
                                .copied()
                                .filter(|i| self.rows[*i].depth > row.depth);
                        }
                    }
                    KeyCode::Space => self.select(Some(id), events),
                    KeyCode::Enter | KeyCode::NumpadEnter => {
                        events.push(TreeEvent::Activated { node: id })
                    }
                    _ => return,
                }
                if let Some(i) = next {
                    let node = self.rows[i].id;
                    if self.focused != Some(node) {
                        self.focused = Some(node);
                        self.scroll = true;
                    }
                    if follows {
                        self.select(Some(node), events);
                    }
                }
            }
        }
    }
}
