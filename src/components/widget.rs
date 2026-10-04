use super::{ContextMenuItem, ContextMenuWidget, Response, Tooltip, TooltipWidget, Ui};

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
}
