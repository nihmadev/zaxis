use super::*;
use crate::{layout::LayoutCursor, Layout};

impl GridUi<'_, '_> {
    pub fn row<R>(
        &mut self,
        source: impl Hash,
        build: impl FnOnce(&mut GridRow<'_, '_, '_>) -> R,
    ) -> R {
        let key = Id::new(source);
        let id = self.id.with(("row", key));
        let y = self.height
            + if self.rows.is_empty() {
                0.0
            } else {
                self.style.spacing.y
            };
        let mut row = GridRow {
            grid: self,
            id,
            y,
            cells: Vec::new(),
            height: 0.0,
        };
        let result = build(&mut row);
        let height = row
            .grid
            .fixed_height
            .unwrap_or(row.height.max(row.grid.style.row_min_height));
        row.grid.height = y + height;
        row.grid.rows.push(Row {
            y,
            height,
            cells: row.cells,
            key,
        });
        result
    }
}
pub struct GridRow<'a, 'ui, 'ctx> {
    grid: &'a mut GridUi<'ui, 'ctx>,
    id: Id,
    y: f32,
    cells: Vec<Cell>,
    height: f32,
}
impl GridRow<'_, '_, '_> {
    /// The callback runs once. Its response coordinates are the build coordinates;
    /// use GridOutput::cells for final cell bounds after content sizing/alignment.
    pub fn cell<R>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let index = self.cells.len();
        let hidden = index >= self.grid.columns.len();
        if hidden {
            self.grid.ui.context.report(
                crate::DiagnosticKind::InvalidUsage,
                Some(self.id),
                None,
                || "more cells than Grid columns; the extra cell is not shown".into(),
            );
        }
        // An extra cell is laid out against the last column but never shown.
        let i = index.min(self.grid.columns.len().saturating_sub(1));
        let c = &self.grid.columns[i];
        let id = self.id.with(c.id);
        let padding = self.grid.style.cell_padding;
        let x = self.grid.widths[..i].iter().sum::<f32>() + self.grid.style.spacing.x * i as f32;
        let offset = self
            .grid
            .offsets
            .get(&id)
            .copied()
            .unwrap_or(Vec2::new(x + padding.left.max(0.0), padding.top.max(0.0)));
        let origin = self.grid.origin + Vec2::new(0.0, self.y) + offset;
        let width = if c.width == ColumnWidth::Content {
            (self.grid.available - self.grid.style.padding.size().x - padding.size().x)
                .max(self.grid.widths[i] - padding.size().x)
                .max(0.0)
        } else {
            (self.grid.widths[i] - padding.size().x).max(0.0)
        };
        let height = self
            .grid
            .fixed_height
            .map_or(self.grid.ui.available_height(), |h| {
                (h - padding.size().y).max(0.0)
            });
        let bounds = if hidden {
            Rect::default()
        } else {
            Rect::from_min_size(origin, Vec2::new(width, height))
        };
        let ui = &mut self.grid.ui;
        let spacing = self.grid.style.component_spacing.max(0.0);
        ui.context.begin_placement(ui.window);
        let mut child = Ui {
            flow: None,
            context: ui.context,
            window: ui.window,
            scope: id,
            sequence: 0,
            clip: Rect::from_min_size(Vec2::splat(-1.0e9), Vec2::splat(2.0e9)),
            layout: LayoutCursor::new(bounds, Layout::Vertical, spacing),
            enabled: ui.enabled,
            backdrop_blur: ui.backdrop_blur,
            hover_style: ui.hover_style,
            local_style: ui.local_style.clone(),
            local_style_revision: ui.local_style_revision,
        };
        child.begin_layout(crate::Align::Start);
        let result = if self.grid.columns[i].tabular {
            let patch = crate::StyleOverrides {
                text: crate::TextStyle {
                    tabular_numbers: Some(true),
                    ..Default::default()
                },
                ..Default::default()
            };
            child.with_style(&patch, build)
        } else {
            build(&mut child)
        };
        child.finish_layout();
        let size = child.layout.used;
        let placement = child.context.end_placement();
        if hidden {
            child.context.place(placement, Vec2::ZERO, Rect::default());
            return result;
        }
        self.height = self.height.max(size.y + padding.size().y);
        self.grid.measured[i] = self.grid.measured[i].max(size.x + padding.size().x);
        self.cells.push(Cell {
            id,
            origin,
            size,
            placement,
        });
        result
    }
}
