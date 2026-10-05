use super::Ui;
use crate::Id;
use std::hash::Hash;

/// A group of [`SelectableLabel`](super::SelectableLabel)s that select as one document.
///
/// A drag that starts in one label and ends in another selects the end of the first, every
/// label between them, and the start of the last, in the order the labels were built.
/// Ctrl/Cmd+A selects the whole group and Ctrl/Cmd+C copies it with the labels joined by a
/// line break (or [`SelectionScope::separator`]). A label outside any scope is a group of one.
///
/// Only labels that were built take part: a virtualized list builds just its visible rows, so
/// rows that were never drawn are not selected and not copied.
///
/// ```
/// # use zaxis::*;
/// # let mut ctx = Context::new();
/// # ctx.run(|c| { Root::new().show(c, |ui| {
/// ui.selection_scope(|ui| {
///     ui.selectable_label("First paragraph");
///     ui.selectable_label("Second paragraph");
/// });
/// # }); });
/// ```
pub struct SelectionScope {
    id: Option<Id>,
    separator: String,
    copy_urls: bool,
}

impl SelectionScope {
    pub fn new() -> Self {
        Self {
            id: None,
            separator: "\n".into(),
            copy_urls: false,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    /// Text put between the copied labels; a line break by default.
    pub fn separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = separator.into();
        self
    }
    /// Copy ` (url)` after each link of a copied selection.
    pub fn copy_url_in_selection(mut self, copy: bool) -> Self {
        self.copy_urls = copy;
        self
    }

    pub fn show<R>(self, ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let id = match self.id {
            Some(source) => ui.scope.with(("selection-scope", source)),
            None => ui.next_id("selection-scope"),
        };
        let scope = ui.context.selection.scopes.entry(id).or_default();
        scope.separator = self.separator;
        scope.copy_urls = self.copy_urls;
        ui.context.selection.stack.push(id);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(ui)));
        ui.context.selection.stack.pop();
        ui.context.selection_commit(id);
        match result {
            Ok(value) => value,
            Err(error) => std::panic::resume_unwind(error),
        }
    }
}

impl Default for SelectionScope {
    fn default() -> Self {
        Self::new()
    }
}

impl Ui<'_> {
    /// Group selectable labels into one selection; see [`SelectionScope`].
    pub fn selection_scope<R>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        SelectionScope::new().show(self, build)
    }
}
