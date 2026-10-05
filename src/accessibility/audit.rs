//! Usage problems of the collected nodes, reported through the ordinary diagnostics.

use super::node::{HIDDEN, UNNAMED_OK};
use crate::{Context, DiagnosticKind};
use std::collections::HashSet;

impl Context {
    /// Problems of this pass's nodes, found before diagnostics are published: controls a
    /// screen reader could not name, and two nodes with one id.
    pub(crate) fn audit_accessibility(&mut self) {
        if !self.a11y.active {
            return;
        }
        let mut seen = HashSet::with_capacity(self.a11y.nodes.len());
        let mut nameless = Vec::new();
        let mut duplicate = Vec::new();
        for (node, slot) in self.a11y.nodes.iter().zip(&self.a11y.slots) {
            if slot.removed {
                continue;
            }
            if !seen.insert(node.id) {
                duplicate.push(node.id);
            }
            let named = node.label.is_some()
                || node
                    .more
                    .as_ref()
                    .is_some_and(|m| !m.labelled_by.is_empty() || m.placeholder.is_some());
            if node.role.needs_name() && !named && !node.has(HIDDEN | UNNAMED_OK) {
                nameless.push((node.id, node.role));
            }
        }
        for id in duplicate {
            self.report(DiagnosticKind::IdCollision, Some(id), None, || {
                "duplicate id in the accessibility tree: use Ui::push_id or an id_source".into()
            });
        }
        for (id, role) in nameless {
            self.report(
                DiagnosticKind::MissingAccessibleName,
                Some(id),
                None,
                || {
                    format!(
                        "{role:?} has no accessible name: add a label or `.accessible_label(..)`"
                    )
                },
            );
        }
    }
}
