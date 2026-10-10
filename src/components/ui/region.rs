//! Explicit positioning of ordinary controls without allocating space in the parent.
use super::Ui;
use crate::{layout::LayoutCursor, Align, Id, Layout, Rect};
use std::hash::Hash;

impl Ui<'_> {
    /// Build once in a vertical UI starting at `rect.min`, in this UI's coordinates.
    /// Clips to `rect`; does not advance the parent cursor. Give each region a stable source.
    /// Inside PanZoom these are content coordinates, including negative positions.
    #[track_caller]
    pub fn at<R>(
        &mut self,
        source: impl Hash,
        rect: Rect,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        let rect = super::super::sanitize::positioned_rect("Ui::at", rect).unwrap_or_default();
        self.region(
            self.scope.with(("region", Id::new(source))),
            rect,
            self.clip.intersect(rect),
            self.enabled,
            build,
        )
    }

    pub(crate) fn region<R>(
        &mut self,
        scope: Id,
        rect: Rect,
        clip: Rect,
        enabled: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        let spacing = self.style().spacing.max(0.0);
        let mut child = Ui {
            context: &mut *self.context,
            window: self.window,
            scope,
            sequence: 0,
            clip,
            layout: LayoutCursor::new(rect, Layout::Vertical, spacing),
            enabled: self.enabled && enabled,
            backdrop_blur: self.backdrop_blur,
            hover_style: self.hover_style,
            flow: None,
            local_style: self.local_style.clone(),
            local_style_revision: self.local_style_revision,
        };
        child.begin_layout(Align::Start);
        let result = build(&mut child);
        child.finish_layout();
        result
    }
}
