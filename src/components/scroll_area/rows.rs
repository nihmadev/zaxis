use super::{ScrollArea, ScrollAreaOutput, Ui};
use crate::{layout::LayoutCursor, Layout, Rect, Vec2};
use std::ops::Range;

/// Vertical geometry of a virtualized row list. The engine asks only for the rows that
/// intersect the viewport; implementations answer in O(1) or O(log n).
pub trait RowMetrics {
    fn count(&self) -> usize;
    fn total_height(&self) -> f32;
    /// Content y of the row's top edge.
    fn top(&self, index: usize) -> f32;
    fn height(&self, index: usize) -> f32;
    /// The row that contains content y, clamped to the last row.
    fn row_at(&self, y: f32) -> usize;
    /// Indices of rows that start a section (sticky headers), ascending.
    fn sections(&self) -> &[usize] {
        &[]
    }
    /// Extra pixels built above and below the viewport.
    fn overscan(&self) -> f32 {
        0.0
    }
    /// Rows are measured by building them: their height is not known up front.
    fn measured(&self) -> bool {
        false
    }
    /// Record a row's built height; returns how much it differs from the old one.
    fn measure(&mut self, _index: usize, _height: f32) -> f32 {
        0.0
    }
}

/// What the shared engine asks its builder to do.
pub(crate) enum RowPass<'m> {
    /// Fill one row; `bounds` has the metrics' current height.
    Row { index: usize, bounds: Rect },
    /// Once, after the rows, for overlays. `visible` is the viewport in row coordinates.
    Overlay {
        visible: Rect,
        rows: &'m dyn RowMetrics,
    },
}

/// Every row has the same pitch.
pub(crate) struct FixedRows {
    pub pitch: f32,
    pub count: usize,
}
impl RowMetrics for FixedRows {
    fn count(&self) -> usize {
        self.count
    }
    fn total_height(&self) -> f32 {
        let height = self.pitch * self.count as f32;
        if height.is_finite() {
            height
        } else {
            f32::MAX / 4.0
        }
    }
    fn top(&self, index: usize) -> f32 {
        self.pitch * index as f32
    }
    fn height(&self, _: usize) -> f32 {
        self.pitch
    }
    fn row_at(&self, y: f32) -> usize {
        ((y.max(0.0) / self.pitch).floor() as usize).min(self.count.saturating_sub(1))
    }
}

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
        self,
        ui: &mut Ui<'_>,
        row_height: f32,
        total_rows: usize,
        key: impl FnMut(usize) -> K,
        mut build: impl FnMut(&mut Ui<'_>, usize),
    ) -> ScrollAreaOutput<Range<usize>> {
        let pitch =
            crate::components::sanitize::positive("ScrollArea::show_rows row_height", row_height)
                .unwrap_or(ui.style().control_height.max(1.0));
        let mut rows = FixedRows {
            pitch,
            count: total_rows,
        };
        self.show_rows_laid_out(ui, &mut rows, key, |ui, pass| {
            if let RowPass::Row { index, .. } = pass {
                build(ui, index);
            }
            None
        })
    }

    /// The shared virtualization core. `build` fills one row and returns the height it
    /// actually used when rows are [`measured`](RowMetrics::measured). Rows that start above
    /// the viewport and change size keep their bottom edge in place (the offset follows), so
    /// measuring never moves what the user is looking at.
    pub(crate) fn show_rows_laid_out<K: std::hash::Hash, M: RowMetrics>(
        mut self,
        ui: &mut Ui<'_>,
        metrics: &mut M,
        mut key: impl FnMut(usize) -> K,
        mut build: impl FnMut(&mut Ui<'_>, RowPass<'_>) -> Option<f32>,
    ) -> ScrollAreaOutput<Range<usize>> {
        if !self.axes[1] {
            ui.context
                .report(crate::DiagnosticKind::InvalidUsage, None, None, || {
                    "show_rows needs vertical scrolling; enabled".into()
                });
            self.axes[1] = true;
        }
        let measured = metrics.measured();
        self.show_content(ui, Some(metrics.total_height()), |ui| {
            let origin = ui.layout.bounds.min;
            let visible = ui.clip_rect();
            let width = ui.layout.bounds.size().x;
            let count = metrics.count();
            let mut range = 0..0;
            let mut anchor = 0.0;
            if !visible.is_empty() && count > 0 {
                let view_top = (visible.min.y - origin.y).max(0.0);
                let view_bottom = (visible.max.y - origin.y).max(0.0);
                let guard = metrics.overscan();
                let first = metrics.row_at((view_top - guard).max(0.0));
                let mut index = first;
                let mut y = metrics.top(first);
                while index < count && y < view_bottom + guard {
                    let height = metrics.height(index);
                    let bounds =
                        Rect::from_min_size(origin + Vec2::new(0.0, y), Vec2::new(width, height));
                    // Measured rows may grow past their estimate while they are built.
                    let clip = if measured {
                        Rect::from_min_size(bounds.min, Vec2::new(width, 1.0e6))
                    } else {
                        bounds
                    };
                    let mut row = Ui {
                        flow: None,
                        context: ui.context,
                        window: ui.window,
                        scope: ui.scope.with(("row", key(index))),
                        sequence: 0,
                        clip,
                        layout: LayoutCursor::new(bounds, Layout::Vertical, 0.0),
                        enabled: ui.enabled,
                        backdrop_blur: ui.backdrop_blur,
                        hover_style: ui.hover_style,
                        local_style: ui.local_style.clone(),
                        local_style_revision: ui.local_style_revision,
                    };
                    row.begin_layout(crate::Align::Start);
                    let used = build(&mut row, RowPass::Row { index, bounds });
                    row.finish_layout();
                    let delta = used.map_or(0.0, |used| metrics.measure(index, used));
                    if delta != 0.0 {
                        // Hit regions were registered with the old height.
                        row.context.request_repaint();
                        if y < view_top {
                            anchor += delta;
                        }
                    }
                    y += height + delta;
                    index += 1;
                }
                range = first..index;
            }
            if anchor != 0.0 {
                if let Some(state) = ui.context.scrolling.states.get_mut(&ui.scope) {
                    state.offset.y += anchor;
                }
            }
            build(
                ui,
                RowPass::Overlay {
                    visible,
                    rows: &*metrics,
                },
            );
            ui.layout.used = Vec2::new(width, metrics.total_height());
            range
        })
    }
}
