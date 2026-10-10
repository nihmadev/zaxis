//! The focus group scope the library's own controls use, without an accessibility node or
//! an id scope of their own.

use super::*;
use crate::context::focus_group::GroupEnd;
use crate::context::{DiagnosticKind, FocusAxis};

impl Ui<'_> {
    /// Regions that take focus and are registered in `build`, in this UI's layer, form one
    /// focus group: one Tab stop, moved by the arrow keys along `axis`. The scope is
    /// restored when `build` returns early or unwinds. Returns the result and how the
    /// group ended.
    pub(crate) fn focus_scope<R>(
        &mut self,
        id: Id,
        axis: FocusAxis,
        wrap: bool,
        build: impl FnOnce(&mut Self) -> R,
    ) -> (R, GroupEnd) {
        if self.context.focus_groups.begin(id, self.window, axis, wrap) {
            self.context
                .report(DiagnosticKind::IdCollision, Some(id), None, || {
                    "duplicate id: give each focus group a unique id_source or push_id scope".into()
                });
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(self)));
        let end = self.context.focus_groups.end(self.context.focused());
        match result {
            Ok(value) => (value, end),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }
}
