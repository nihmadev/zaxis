use super::super::disclosure::{self, Header};
use super::*;
use crate::{
    context::{HitAction, HitRegion},
    Padding, ScrollArea, ScrollStyle, Shape,
};

impl TreeView<'_> {
    pub(super) fn show_inner(
        mut self,
        ui: &mut Ui<'_>,
        model: &impl TreeModel,
        mut actions: impl FnMut(&mut Ui<'_>, Id),
    ) -> TreeOutput {
        let id = ui.scope.with(("tree", self.id));
        let mut state = ui
            .context
            .trees
            .remove(&id)
            .unwrap_or_else(|| TreeState::new(self.initial_open, self.initial_selected));
        if state.last_frame == ui.context.frame {
            ui.context
                .report(crate::DiagnosticKind::IdCollision, Some(id), None, || {
                    "duplicate id: two TreeViews share one id_source".into()
                });
        }
        state.last_frame = ui.context.frame;
        let action_focus = ui
            .context
            .focused_widget
            .filter(|f| state.action_ids.contains_key(f));
        state.action_ids.clear();
        if let Some(open) = &self.controlled_open {
            if state.open != **open {
                state.open.clone_from(open);
                state.dirty = true;
            }
        }
        if let Some(selected) = &self.controlled_selected {
            state.selected = **selected;
        }
        let mut events = Vec::new();
        let mut rebuilt = false;
        if state.revision != Some(model.revision()) {
            state.dirty = true;
        }
        if state.dirty {
            state.rebuild(model, &mut events);
            rebuilt = true;
        }
        // A controlled selection can be deleted even without a topology change.
        if state.selected.is_some_and(|n| model.node(n).is_none()) {
            state.select(None, &mut events);
        }
        let enabled = self.enabled && ui.enabled;
        if enabled {
            if let Some(node) = self.reveal {
                state.reveal(node, model, &mut events);
                ui.context.set_focus(Some(id));
            }
            let mut inputs = ui.context.tree_input.remove(&id).unwrap_or_default();
            access::requests(ui, id, &state, &mut inputs);
            for input in inputs {
                if state.dirty {
                    state.rebuild(model, &mut events);
                    rebuilt = true;
                }
                state.input(
                    input,
                    model,
                    self.follows_focus,
                    self.double_click_expand,
                    &mut events,
                );
            }
        } else {
            ui.context.tree_input.remove(&id);
        }
        if state.dirty {
            state.rebuild(model, &mut events);
            rebuilt = true;
        }
        let owns_focus = ui.context.has_focus(id);
        if owns_focus && !state.owner_focused {
            state.scroll = true;
            if self.follows_focus {
                state.select(state.focused, &mut events);
            }
        }
        state.owner_focused = owns_focus;
        let mut style = ui.style().tree.clone();
        if let Some(patch) = self.style {
            style.merge(&patch);
        }
        let mut pitch =
            disclosure::dimension(style.row.height.unwrap_or(ui.style().control_height));
        if pitch <= 0.0 {
            ui.context
                .report(crate::DiagnosticKind::InvalidUsage, Some(id), None, || {
                    "TreeView needs a positive fixed row height; using the control height".into()
                });
            pitch = ui.style().control_height.max(1.0);
        }
        let indent = disclosure::dimension(style.indent.unwrap_or(18.0));
        let outer = Rect::from_min_size(
            ui.layout.cursor,
            Vec2::new(
                ui.available_width().max(0.0),
                ui.available_height().min(self.height).max(0.0),
            ),
        );
        // Register the single owner before descendant hits. Disabled rows explicitly block it.
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect: outer,
            clip: ui.clip,
            action: if enabled {
                HitAction::Tree
            } else {
                HitAction::Block
            },
        });
        let owner_focus = ui.context.focus_visible(id);
        let tree = access::begin(ui, id, &self.label, enabled, &state);
        if self.drag {
            drag::keep_alive(ui, id, model);
        }
        let mut scroll = ScrollArea::vertical()
            .id_source(id)
            .a11y_hidden()
            .max_height(self.height)
            .style(style.scroll.unwrap_or(ScrollStyle {
                padding: Padding::default(),
                spacing: 0.0,
                ..ui.style().scroll
            }));
        if state.scroll && ui.context.has_focus(id) {
            if let Some(index) = state.focused.and_then(|n| state.index.get(&n).copied()) {
                scroll = scroll.scroll_to_rect(Rect::from_min_size(
                    Vec2::new(0.0, index as f32 * pitch),
                    Vec2::new(1.0, pitch),
                ));
            }
        }
        state.scroll = false;
        let mut rows_built = 0;
        let total = state.rows.len();
        let rows = &state.rows;
        let output = ui.add_enabled_ui(enabled, |ui| {
            scroll.show_rows_keyed(
                ui,
                pitch,
                total,
                |index| rows[index].id,
                |ui, index| {
                    let row = rows[index];
                    let Some(node) = model.node(row.id) else {
                        return;
                    };
                    rows_built += 1;
                    let row_id = access::row_id(id, row.id);
                    let open = state.open.contains(&row.id);
                    let selected = state.selected == Some(row.id);
                    let item = access::row(ui, row_id, &row, node.label, open, selected, enabled);
                    let h = disclosure::header(
                        ui,
                        Header {
                            id: row_id,
                            label: node.label,
                            icon: node.icon,
                            branch: row.children.expandable(),
                            open,
                            enabled: row.enabled && enabled,
                            selected,
                            focus: owner_focus && state.focused == Some(row.id),
                            indent: row.depth as f32 * indent,
                            action: HitAction::TreeRow {
                                tree: id,
                                node: row.id,
                                chevron: false,
                            },
                            chevron_action: Some(HitAction::TreeRow {
                                tree: id,
                                node: row.id,
                                chevron: true,
                            }),
                            style: &style.row,
                        },
                        |ui| actions(ui, row.id),
                    );
                    access::end_row(ui, item, h.response.rect);
                    if self.drag && row.enabled && enabled {
                        drag::attach(
                            ui,
                            drag::Dragged {
                                tree: id,
                                node: row.id,
                                rows,
                                index: &state.index,
                                response: h.response,
                                indent: row.depth as f32 * indent,
                                branch: row.children.expandable(),
                                focused: owns_focus && state.focused == Some(row.id),
                            },
                            &mut events,
                        );
                    }
                    state
                        .action_ids
                        .extend(h.action_ids.into_iter().map(|a| (a, row.id)));
                    if let Some(guide) = style.guides.filter(|b| b.width > 0.0) {
                        for depth in 0..row.depth {
                            let x = h.chevron.center().x - (row.depth - depth) as f32 * indent;
                            ui.paint(Shape::Line {
                                start: Vec2::new(x, h.response.rect.min.y),
                                end: Vec2::new(x, h.response.rect.max.y),
                                width: guide.width,
                                color: guide.color,
                            });
                        }
                    }
                },
            )
        });
        ui.context.a11y_scroll(&tree, output.id);
        ui.a11y_end(tree, Some(outer));
        if action_focus.is_some_and(|f| !state.action_ids.contains_key(&f)) {
            if ui
                .context
                .popup
                .as_ref()
                .is_some_and(|p| p.return_focus == action_focus)
            {
                ui.context.dismiss_popup(false);
            }
            ui.context.set_focus(Some(id));
            ui.context.request_repaint();
        }
        if let Some(open) = self.controlled_open.as_mut() {
            if **open != state.open {
                open.clone_from(&state.open);
            }
        }
        if let Some(selected) = self.controlled_selected.as_mut() {
            **selected = state.selected;
        }
        if !events.is_empty() {
            ui.context.request_repaint();
        }
        // The same model problems `TreeOutput::issues` lists also reach `Context::diagnostics`.
        for issue in &state.issues {
            ui.context.report(
                crate::DiagnosticKind::InvalidModel,
                Some(match issue {
                    TreeIssue::DuplicateOrCycle(node)
                    | TreeIssue::MissingNode(node)
                    | TreeIssue::InvalidRevealPath(node) => *node,
                }),
                Some(outer),
                || format!("tree model: {issue:?}"),
            );
        }
        let out = TreeOutput {
            id,
            selected: state.selected,
            focused: state.focused,
            events,
            issues: state.issues.clone(),
            visible_rows: output.inner,
            logical_rows: total,
            rows_built,
            rebuilt,
            viewport: output.viewport,
            scroll_offset: output.offset,
        };
        ui.context.trees.insert(id, state);
        out
    }
}
