use super::*;
use crate::{StyleOverrides, Theme};
use std::{
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    sync::Arc,
};

impl Ui<'_> {
    /// Override a subtree without touching Context or the native clear color.
    /// IDs, layout and input remain in the current scope. Restores even on unwind.
    pub fn with_style<R>(
        &mut self,
        patch: &StyleOverrides,
        build: impl FnOnce(&mut Self) -> R,
    ) -> R {
        self.with_effective_style(patch.resolve(self.style()), build)
    }
    /// Replace the base theme for this subtree, then apply its explicit overrides.
    pub fn with_theme<R>(&mut self, theme: &Theme, build: impl FnOnce(&mut Self) -> R) -> R {
        self.with_effective_style(theme.resolve(), build)
    }
    pub(super) fn with_effective_style<R>(
        &mut self,
        style: Style,
        build: impl FnOnce(&mut Self) -> R,
    ) -> R {
        let spacing = self.layout.spacing;
        if style.spacing != self.style().spacing {
            self.layout.spacing = style.spacing.max(0.0);
        }
        let key = self.next_id("style-scope");
        let revision = if let Some((_, revision, _)) =
            self.context.local_styles.get(&key).filter(|s| s.0 == style)
        {
            *revision
        } else {
            self.context.local_style_serial = self.context.local_style_serial.wrapping_add(1);
            self.context.local_style_serial
        };
        self.context
            .local_styles
            .insert(key, (style.clone(), revision, self.context.frame));
        let previous_revision = self.local_style_revision;
        self.local_style_revision = revision;
        let old = self.local_style.replace(Arc::new(style));
        // The scoped value is on this Ui, never on Context. A safe unwind guard
        // via catch_unwind allows callers to recover and continue sibling UI.
        let result = catch_unwind(AssertUnwindSafe(|| build(self)));
        self.local_style = old;
        self.local_style_revision = previous_revision;
        self.layout.spacing = spacing;
        match result {
            Ok(value) => value,
            Err(error) => resume_unwind(error),
        }
    }
}
