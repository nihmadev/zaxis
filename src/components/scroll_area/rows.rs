use super::{ScrollArea, ScrollAreaOutput, Ui};
use crate::{layout::LayoutCursor, Layout, Rect, Vec2};
use std::ops::Range;

impl ScrollArea {
    /// Execute only intersecting fixed-height rows, with stable index scopes.
    /// `row_height` is the full row pitch, including any desired gap. Each row has
    /// its own clipped layout; no widgets are evaluated for skipped rows.
    pub fn show_rows(
        self,
        ui: &mut Ui<'_>,
        row_height: f32,
        total_rows: usize,
        build: impl FnMut(&mut Ui<'_>, usize),
    ) -> ScrollAreaOutput<Range<usize>> {
        self.show_rows_keyed(ui, row_height, total_rows, |index| index, build)
    }

    /// Like show_rows, but row scopes use application keys instead of indices.
    /// Moving a keyed row preserves the identities of every descendant control.
    pub fn show_rows_keyed<K: std::hash::Hash>(
        mut self,
        ui: &mut Ui<'_>,
        row_height: f32,
        total_rows: usize,
        mut key: impl FnMut(usize) -> K,
        mut build: impl FnMut(&mut Ui<'_>, usize),
    ) -> ScrollAreaOutput<Range<usize>> {
        if !self.axes[1] {
            ui.context
                .report(crate::DiagnosticKind::InvalidUsage, None, None, || {
                    "show_rows needs vertical scrolling; enabled".into()
                });
            self.axes[1] = true;
        }
        let row_height =
            crate::components::sanitize::positive("ScrollArea::show_rows row_height", row_height)
                .unwrap_or(ui.style().control_height.max(1.0));
        let height = row_height * total_rows as f32;
        let height = if height.is_finite() {
            height
        } else {
            f32::MAX / 4.0
        };

        self.show_content(ui, Some(height), |ui| {
            let origin = ui.layout.bounds.min;
            let visible = ui.clip_rect();
            let range = if visible.is_empty() {
                0..0
            } else {
                let first = ((visible.min.y - origin.y).max(0.0) / row_height).floor() as usize;
                let end = ((visible.max.y - origin.y).max(0.0) / row_height).ceil() as usize;
                first.min(total_rows)..end.min(total_rows)
            };
            for index in range.clone() {
                let bounds = Rect::from_min_size(
                    origin + Vec2::new(0.0, index as f32 * row_height),
                    Vec2::new(ui.layout.bounds.size().x, row_height),
                );
                let mut row = Ui {
                    flow: None,
                    context: ui.context,
                    window: ui.window,
                    scope: ui.scope.with(("row", key(index))),
                    sequence: 0,
                    clip: bounds,
                    layout: LayoutCursor::new(bounds, Layout::Vertical, 0.0),
                    enabled: ui.enabled,
                    backdrop_blur: ui.backdrop_blur,
                    hover_style: ui.hover_style,
                    local_style: ui.local_style.clone(),
                    local_style_revision: ui.local_style_revision,
                };
                row.begin_layout(crate::Align::Start);
                build(&mut row, index);
                row.finish_layout();
            }
            ui.layout.used = Vec2::new(ui.layout.bounds.size().x, height);
            range
        })
    }
}
