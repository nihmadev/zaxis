//! Naming, describing and hiding any widget from the call site.

use super::{node::HIDDEN, AccessRole};
use crate::{Response, Ui, Widget};

/// A widget with its accessible name, description or role replaced; made by
/// [`Widget::accessible_label`] and its siblings.
pub struct Accessible<W> {
    widget: W,
    label: Option<String>,
    description: Option<String>,
    role: Option<AccessRole>,
    hidden: bool,
}

impl<W> Accessible<W> {
    pub(crate) fn new(widget: W) -> Self {
        Self {
            widget,
            label: None,
            description: None,
            role: None,
            hidden: false,
        }
    }
    pub(crate) fn label(mut self, label: String) -> Self {
        self.label = Some(label);
        self
    }
    pub(crate) fn description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
    pub(crate) fn role(mut self, role: AccessRole) -> Self {
        self.role = Some(role);
        self
    }
    pub(crate) fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }
}

impl<W: Widget> Widget for Accessible<W> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let start = ui.context.a11y_len();
        let response = self.widget.ui(ui);
        if !ui.context.a11y_on() {
            return response;
        }
        if self.hidden {
            for slot in &mut ui.context.a11y.slots[start..] {
                slot.removed = true;
            }
            return response;
        }
        if ui.context.a11y_len() == start {
            // A widget that does not describe itself: publish it as what the caller says.
            ui.a11y(response.id, response.rect, AccessRole::Group, |node| {
                node.click = Some(response.id);
            });
        }
        if let Some(node) = ui.context.a11y_node_mut(start) {
            if let Some(role) = self.role {
                node.role = role;
            }
            if let Some(label) = self.label {
                node.label(label);
            }
            if let Some(description) = self.description {
                node.description(description);
            }
            node.flags &= !HIDDEN;
        }
        response
    }
}
