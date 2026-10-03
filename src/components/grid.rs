//! Shared columns with single-pass arbitrary cell callbacks.
mod rows;
use super::{
    columns::{dimension, resolve},
    Column, ColumnWidth, Ui,
};
use crate::{context::placement::Placement, Color, Id, Padding, Rect, Vec2};
pub use rows::GridRow;
use std::{collections::HashMap, hash::Hash};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GridStyle {
    pub surface: crate::SurfaceStyle,

    pub padding: Padding,
    pub cell_padding: Padding,
    pub spacing: Vec2,
    pub component_spacing: f32,
    pub row_min_height: f32,
    pub fill: Color,
    pub rounding: f32,
}
impl Default for GridStyle {
    fn default() -> Self {
        Self {
            surface: Default::default(),

            padding: Padding::all(10.0),
            cell_padding: Padding::symmetric(8.0, 4.0),
            spacing: Vec2::new(8.0, 6.0),
            component_spacing: 6.0,
            row_min_height: 0.0,
            fill: Color::gray(51),
            rounding: 8.0,
        }
    }
}
#[derive(Default)]
pub(crate) struct GridState {
    pub(crate) last_frame: u64,
    measured: HashMap<Id, f32>,
    offsets: HashMap<Id, Vec2>,
}
pub struct Grid {
    id: Id,
    columns: Vec<Column>,
    style: Option<GridStyle>,
    width: Option<f32>,
    resolved: Option<Vec<f32>>,
    fixed_height: Option<f32>,
    spacing: Option<Vec2>,
    padding: Option<Padding>,
    cell_padding: Option<Padding>,
    fill: Option<Color>,
}
pub struct GridOutput<R> {
    pub inner: R,
    pub rect: Rect,
    pub widths: Vec<f32>,
    /// Intrinsic widths measured on this pass, including padding.
    pub measured_widths: Vec<f32>,
    pub rows: Vec<Rect>,
    pub cells: Vec<Vec<Rect>>,
}
impl Grid {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            columns: Vec::new(),
            style: None,
            width: None,
            resolved: None,
            fixed_height: None,
            spacing: None,
            padding: None,
            cell_padding: None,
            fill: None,
        }
    }
    pub fn column(mut self, column: Column) -> Self {
        self.columns.push(column);
        self
    }
    pub fn columns(mut self, columns: impl IntoIterator<Item = Column>) -> Self {
        self.columns.extend(columns);
        self
    }
    pub fn style(mut self, style: GridStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(dimension(width));
        self
    }
    pub fn spacing(mut self, spacing: Vec2) -> Self {
        dimension(spacing.x);
        dimension(spacing.y);
        self.spacing = Some(spacing);
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn cell_padding(mut self, padding: Padding) -> Self {
        self.cell_padding = Some(padding);
        self
    }
    pub fn fill(mut self, fill: Color) -> Self {
        self.fill = Some(fill);
        self
    }
    pub(super) fn resolved(mut self, widths: Vec<f32>, height: Option<f32>) -> Self {
        self.resolved = Some(widths);
        self.fixed_height = height;
        self
    }
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut GridUi<'_, '_>) -> R,
    ) -> GridOutput<R> {
        ui.layout_item(|ui| self.show_content(ui, build))
    }

    fn show_content<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut GridUi<'_, '_>) -> R,
    ) -> GridOutput<R> {
        let id = ui.scope.with(("grid", self.id));
        let mut style = self.style.unwrap_or(ui.style().grid);
        if let Some(spacing) = self.spacing {
            style.spacing = spacing;
        }
        if let Some(padding) = self.padding {
            style.padding = padding;
        }
        if let Some(padding) = self.cell_padding {
            style.cell_padding = padding;
        }
        if let Some(fill) = self.fill {
            style.fill = fill;
        }
        dimension(style.spacing.x);
        dimension(style.spacing.y);
        dimension(style.row_min_height);
        assert!(
            !self.columns.is_empty(),
            "Grid requires at least one column"
        );
        let mut unique = std::collections::HashSet::new();
        assert!(
            self.columns.iter().all(|c| unique.insert(c.id)),
            "duplicate column ID"
        );
        let available = self
            .width
            .unwrap_or(ui.available_width())
            .min(ui.available_width());
        let origin = ui.layout.cursor;
        let state = ui.context.grids.remove(&id).unwrap_or_default();
        let measured: Vec<_> = self
            .columns
            .iter()
            .map(|c| state.measured.get(&c.id).copied().unwrap_or(0.0))
            .collect();
        let widths = self.resolved.clone().unwrap_or_else(|| {
            resolve(
                &self.columns,
                &measured,
                (available - style.padding.size().x).max(0.0),
                style.spacing.x,
            )
        });
        let mut grid = GridUi {
            ui,
            id,
            columns: self.columns,
            style,
            origin: origin + Vec2::new(style.padding.left.max(0.0), style.padding.top.max(0.0)),
            available,
            widths,
            measured: vec![0.0; measured.len()],
            offsets: state.offsets,
            next_offsets: HashMap::new(),
            rows: Vec::new(),
            height: 0.0,
            fixed_height: self.fixed_height,
        };
        let inner = build(&mut grid);
        let widths = self.resolved.unwrap_or_else(|| {
            resolve(
                &grid.columns,
                &grid.measured,
                (available - style.padding.size().x).max(0.0),
                style.spacing.x,
            )
        });
        if widths != grid.widths {
            grid.ui.context.request_repaint();
        }
        let content_width =
            widths.iter().sum::<f32>() + style.spacing.x * widths.len().saturating_sub(1) as f32;
        let size = Vec2::new(
            content_width + style.padding.size().x,
            grid.height + style.padding.size().y,
        );
        let rect = Rect::from_min_size(origin, size);
        let ui = &mut grid.ui;
        let effective = ui.style().clone();
        let mut body = super::appearance::Appearance::new(
            style.fill,
            crate::Border::NONE,
            effective.text_color,
        );
        body.rounding = crate::CornerRadius::all(style.rounding);
        body.blur = 0.0;
        body.opacity = effective.opacity;
        body.apply(style.surface);
        let mut paint = Vec::new();
        body.paint_shadow(rect, body.rounding, &mut paint);
        body.paint_body(rect, body.rounding, &effective, body.blur, &mut paint);
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            ui.clip.intersect(rect),
            crate::Blur::new(rect)
                .radius(body.blur)
                .corner_radius(body.rounding),
        );
        ui.context.paint(
            id.with("background"),
            ui.window,
            ui.clip.intersect(rect),
            paint,
        );
        let mut row_rects = Vec::new();
        let mut cells = Vec::new();
        for row in grid.rows {
            let row_rect = Rect::from_min_size(
                grid.origin + Vec2::new(0.0, row.y),
                Vec2::new(content_width, row.height),
            );
            row_rects.push(row_rect);
            let mut x = row_rect.min.x;
            let mut cell_rects = Vec::new();
            for (i, cell) in row.cells.into_iter().enumerate() {
                let bounds = Rect::from_min_size(
                    Vec2::new(x, row_rect.min.y),
                    Vec2::new(widths[i], row.height),
                );
                let padded = style.cell_padding.inset(bounds);
                let column = &grid.columns[i];
                let target = padded.min
                    + Vec2::new(
                        column.horizontal.offset(padded.size().x - cell.size.x),
                        column.vertical.offset(padded.size().y - cell.size.y),
                    );
                if target != cell.origin {
                    grid.ui.context.request_repaint();
                }
                grid.next_offsets.insert(cell.id, target - row_rect.min);
                grid.ui.context.place(
                    cell.placement,
                    target - cell.origin,
                    grid.ui.clip.intersect(bounds),
                );
                cell_rects.push(bounds);
                x += widths[i] + style.spacing.x;
            }
            cells.push(cell_rects);
        }
        grid.ui.context.grids.insert(
            id,
            GridState {
                last_frame: grid.ui.context.frame,
                measured: grid
                    .columns
                    .iter()
                    .zip(&grid.measured)
                    .map(|(c, &m)| (c.id, m))
                    .collect(),
                offsets: grid.next_offsets,
            },
        );
        grid.ui.allocate_space(size);
        GridOutput {
            inner,
            rect,
            widths,
            measured_widths: grid.measured,
            rows: row_rects,
            cells,
        }
    }
}
struct Cell {
    id: Id,
    origin: Vec2,
    size: Vec2,
    placement: Placement,
}
struct Row {
    y: f32,
    height: f32,
    cells: Vec<Cell>,
}
/// Rows are identified by application IDs, never by their display positions.
pub struct GridUi<'a, 'ctx> {
    pub(super) ui: &'a mut Ui<'ctx>,
    id: Id,
    columns: Vec<Column>,
    style: GridStyle,
    origin: Vec2,
    available: f32,
    widths: Vec<f32>,
    measured: Vec<f32>,
    offsets: HashMap<Id, Vec2>,
    next_offsets: HashMap<Id, Vec2>,
    rows: Vec<Row>,
    height: f32,
    fixed_height: Option<f32>,
}
impl Ui<'_> {
    pub fn grid<R>(
        &mut self,
        source: impl Hash,
        columns: impl IntoIterator<Item = Column>,
        build: impl FnOnce(&mut GridUi<'_, '_>) -> R,
    ) -> GridOutput<R> {
        Grid::new(source).columns(columns).show(self, build)
    }
}
