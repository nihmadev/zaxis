use super::{ContextMenuItem, ContextMenuWidget, Response, Tooltip, TooltipWidget, Ui};
use crate::accessibility::{AccessRole, Accessible};

/// An immediate mode component that allocates and paints itself in a UI.
pub trait Widget {
    fn ui(self, ui: &mut Ui<'_>) -> Response;

    /// Show a tooltip while the widget is hovered, in the same expression:
    /// `ui.add(Button::new("Save").tooltip("Write the file"))`.
    fn tooltip(self, text: impl Into<String>) -> TooltipWidget<Self>
    where
        Self: Sized,
    {
        Tooltip::new(text).wrap(self)
    }

    /// Open a context menu on secondary click. The chosen item is reported by
    /// [`Response::menu_selected`] on the pass it is picked.
    fn context_menu(self, items: &[ContextMenuItem]) -> ContextMenuWidget<'_, Self>
    where
        Self: Sized,
    {
        ContextMenuWidget::new(self, items)
    }

    /// The name a screen reader speaks for this widget, replacing the one it derives from
    /// its visible text. Required for controls without text: an icon button, a bare
    /// switch, a slider without a caption.
    fn accessible_label(self, label: impl Into<String>) -> Accessible<Self>
    where
        Self: Sized,
    {
        Accessible::new(self).label(label.into())
    }

    /// Extra detail a screen reader speaks after the name: a hint or the reason for an error.
    fn accessible_description(self, description: impl Into<String>) -> Accessible<Self>
    where
        Self: Sized,
    {
        Accessible::new(self).description(description.into())
    }

    /// Present this widget to assistive technology as `role`. A custom widget that does
    /// not describe itself becomes a node of this role with the response's bounds.
    fn accessible_role(self, role: AccessRole) -> Accessible<Self>
    where
        Self: Sized,
    {
        Accessible::new(self).role(role)
    }

    /// Leave this widget and everything in it out of the accessibility tree: decoration
    /// that says nothing a screen reader user needs.
    fn accessibility_hidden(self) -> Accessible<Self>
    where
        Self: Sized,
    {
        Accessible::new(self).hidden()
    }
}
