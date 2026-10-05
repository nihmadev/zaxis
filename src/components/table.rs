//! Data-agnostic table composition, sharing Grid columns and ScrollArea routing.
mod access;
mod body;
mod header;
mod style;
use super::{
    columns::{dimension, resolve},
    Column, GridRow, ScrollArea, ScrollStyle, Ui,
};
use crate::{Border, CornerRadius, Id, Padding, Rect, Vec2};
pub use body::TableBody;
use std::{collections::HashMap, hash::Hash, ops::Range};
pub use style::TableStyle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SortRequest {
    pub column: Id,
    pub direction: SortDirection,
}
#[derive(Default)]
pub(crate) struct TableState {
    pub last_frame: u64,
    pub widths: HashMap<Id, f32>,
    pub current_widths: HashMap<Id, f32>,
    pub resize_columns: std::collections::HashSet<Id>,
    measured: HashMap<Id, f32>,
    selected: Option<Id>,
    sort: Option<SortRequest>,
    pub drag: Option<(Id, f32)>,
}
pub struct Table {
    id: Id,
    columns: Vec<Column>,
    style: Option<TableStyle>,
    rounding: Option<CornerRadius>,
    border: Option<Border>,
    padding: Option<Padding>,
    max_height: f32,
    width: Option<f32>,
    selected: Option<Option<Id>>,
    sort: Option<Option<SortRequest>>,
    offset: Option<Vec2>,
    striped: Option<bool>,
    separators: Option<bool>,
    selectable: bool,
    resizable: bool,
    drag_rows: bool,
    label: String,
}
pub struct TableOutput<R> {
    pub inner: R,
    pub rect: Rect,
    pub header_rect: Rect,
    pub viewport: Rect,
    pub widths: Vec<f32>,
    pub offset: Vec2,
    pub selected_row: Option<Id>,
    pub row_clicked: Option<Id>,
    pub sort_request: Option<SortRequest>,
    /// A row was dragged before or after another (see [`Table::drag_rows`]).
    pub row_moved: Option<crate::RowMove>,
    /// Only built rows; virtualized output stays proportional to visible content.
    pub rows: Vec<(Id, Rect)>,
}
impl Table {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            columns: Vec::new(),
            style: None,
            rounding: None,
            border: None,
            padding: None,
            max_height: 300.0,
            width: None,
            selected: None,
            sort: None,
            offset: None,
            striped: None,
            separators: None,
            selectable: true,
            resizable: true,
            drag_rows: false,
            label: String::new(),
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
    pub fn style(mut self, style: TableStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
    }
    #[deprecated(
        note = "use `.corner_radius(..)`; one name for the corner radius of every component"
    )]
    pub fn rounding(self, rounding: impl Into<CornerRadius>) -> Self {
        self.corner_radius(rounding)
    }
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = dimension(height);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(dimension(width));
        self
    }
    pub fn selected_row(mut self, selected: Option<Id>) -> Self {
        self.selected = Some(selected);
        self
    }
    pub fn sort(mut self, sort: Option<SortRequest>) -> Self {
        self.sort = Some(sort);
        self
    }
    #[track_caller]
    pub fn scroll_offset(mut self, offset: Vec2) -> Self {
        if let Some(offset) = super::sanitize::finite_vec2("Table::scroll_offset", offset) {
            self.offset = Some(offset.max(Vec2::ZERO));
        }
        self
    }
    pub fn striped(mut self, striped: bool) -> Self {
        self.striped = Some(striped);
        self
    }
    pub fn separators(mut self, separators: bool) -> Self {
        self.separators = Some(separators);
        self
    }
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }
    /// Let rows be dragged to reorder. Rows are sources with `RowDrag` payloads
    /// and before/after targets; the result is `TableOutput::row_moved`. The model
    /// is yours to change. With `show_rows`, call `Ui::keep_drag_source` for the
    /// dragged row while it exists so scrolling it out of view does not end the drag.
    pub fn drag_rows(mut self, drag: bool) -> Self {
        self.drag_rows = drag;
        self
    }
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut TableBody<'_, '_>) -> R,
    ) -> TableOutput<R> {
        self.show_impl(ui, |scroll, ui, setup| {
            let out = scroll.show(ui, |ui| {
                let mut body = TableBody::new(ui, setup, None);
                build(&mut body)
            });
            (out.inner, out.viewport, out.offset)
        })
    }
    /// Fixed-height virtualization. Call `body.row(stable_id, ...)` exactly once
    /// per callback. Display indices are never used for cell/control state.
    pub fn show_rows(
        self,
        ui: &mut Ui<'_>,
        row_height: f32,
        total: usize,
        mut build: impl FnMut(&mut TableBody<'_, '_>, usize),
    ) -> TableOutput<Range<usize>> {
        let row_height = super::sanitize::positive("Table::show_rows row_height", row_height)
            .unwrap_or(ui.style().control_height.max(1.0));
        self.show_impl(ui, |scroll, ui, setup| {
            setup.total = Some(total);
            let out = scroll.show_rows(ui, row_height, total, |ui, index| {
                let before = setup.rows.len();
                let mut body = TableBody::new(ui, setup, Some(row_height));
                body.index = index;
                build(&mut body, index);
                if setup.rows.len() != before + 1 {
                    ui.context
                        .report(crate::DiagnosticKind::InvalidUsage, None, None, || {
                            "Table::show_rows must build exactly one row per callback".into()
                        });
                }
            });
            (out.inner, out.viewport, out.offset)
        })
    }
    fn show_impl<R>(
        self,
        ui: &mut Ui<'_>,
        run: impl FnOnce(ScrollArea, &mut Ui<'_>, &mut body::BodySetup) -> (R, Rect, Vec2),
    ) -> TableOutput<R> {
        ui.layout_item(|ui| self.show_content(ui, run))
    }

    fn show_content<R>(
        mut self,
        ui: &mut Ui<'_>,
        run: impl FnOnce(ScrollArea, &mut Ui<'_>, &mut body::BodySetup) -> (R, Rect, Vec2),
    ) -> TableOutput<R> {
        let id = ui.scope.with(("table", self.id));
        if self.columns.is_empty() {
            ui.context
                .report(crate::DiagnosticKind::InvalidUsage, Some(id), None, || {
                    "Table has no columns; using one flexible column".into()
                });
            self.columns
                .push(super::Column::remainder("table-fallback-column"));
        }
        let mut unique = std::collections::HashSet::new();
        for column in &self.columns {
            if !unique.insert(column.id) {
                ui.context.report(
                    crate::DiagnosticKind::IdCollision,
                    Some(column.id),
                    None,
                    || "duplicate id: two Table columns share one id".into(),
                );
            }
        }
        let mut state = ui.context.tables.remove(&id).unwrap_or_default();
        state.last_frame = ui.context.frame;
        if let Some(selected) = self.selected {
            state.selected = selected;
        }
        if let Some(sort) = self.sort {
            state.sort = sort;
        }
        let mut style = self.style.unwrap_or(ui.style().table);
        if let Some(rounding) = self.rounding {
            style.rounding = rounding;
        }
        if let Some(border) = self.border {
            style.border = border;
        }
        if let Some(padding) = self.padding {
            style.padding = padding;
        }
        if let Some(striped) = self.striped {
            style.striped = striped;
        }
        if let Some(separators) = self.separators {
            style.separators = separators;
        }
        style.header_height = dimension(style.header_height);
        style.row_min_height = dimension(style.row_min_height);
        let available = self
            .width
            .unwrap_or(ui.available_width())
            .min(ui.available_width());
        let size = Vec2::new(available, self.max_height.min(ui.available_height()));
        let rect = ui.allocate_space(size);
        ui.context.paint(
            id.with("fill"),
            ui.window,
            ui.clip.intersect(rect),
            vec![crate::context::Paint::Shape(
                crate::Shape::rect(rect, style.fill)
                    .corner_radius(style.rounding)
                    .border(style.border)
                    .into(),
            )],
        );
        let bounds = content_bounds(rect, style);
        header::resize(
            ui,
            id,
            &self.columns,
            &mut state,
            self.resizable && ui.enabled,
        );
        let measured: Vec<_> = self
            .columns
            .iter()
            .map(|c| {
                let title = ui
                    .context
                    .measure_text(
                        &c.title,
                        super::font_size(style.font_size),
                        style.header_font_weight,
                        f32::INFINITY,
                    )
                    .x
                    + style.cell_padding.size().x
                    + if c.sortable { 18.0 } else { 0.0 };
                state.measured.get(&c.id).copied().unwrap_or(0.0).max(title)
            })
            .collect();
        let mut policies = self.columns.clone();
        for c in &mut policies {
            if let Some(&width) = state.widths.get(&c.id) {
                c.width = super::ColumnWidth::Fixed(width);
            }
        }
        let widths = resolve(&policies, &measured, bounds.size().x, 0.0);
        let content_width = widths.iter().sum::<f32>();
        let header_rect = Rect::from_min_size(
            bounds.min,
            Vec2::new(bounds.size().x, style.header_height.min(bounds.size().y)),
        );
        let body_rect = Rect::from_min_max(Vec2::new(bounds.min.x, header_rect.max.y), bounds.max);
        let nodes = access::begin(ui, id, &self.label, self.columns.len());
        let mut child = Ui {
            flow: None,
            context: ui.context,
            window: ui.window,
            scope: id,
            sequence: 0,
            clip: ui.clip.intersect(body_rect),
            layout: crate::layout::LayoutCursor::new(body_rect, crate::Layout::Vertical, 0.0),
            enabled: ui.enabled,
            backdrop_blur: ui.backdrop_blur,
            hover_style: ui.hover_style,
            local_style: ui.local_style.clone(),
            local_style_revision: ui.local_style_revision,
        };
        let mut scroll = ScrollArea::both()
            .id_source("body")
            .a11y_hidden()
            .max_height(body_rect.size().y)
            .content_width(content_width)
            .style(ScrollStyle {
                padding: Padding::all(0.0),
                spacing: 0.0,
                ..style.scroll
            })
            .overlay_scrollbars(true);
        if let Some(offset) = self.offset {
            scroll = scroll.scroll_offset(offset);
        }
        let mut setup = body::BodySetup {
            id,
            columns: self.columns.clone(),
            widths: widths.clone(),
            style,
            selected: state.selected,
            selectable: self.selectable,
            clicked: None,
            measured: measured.clone(),
            rows: Vec::new(),
            total: None,
            seen: Default::default(),
            origin: None,
            drag: self.drag_rows,
            moved: None,
        };
        let (inner, viewport, offset) = run(scroll, &mut child, &mut setup);
        if let Some(origin) = setup.origin {
            let correction = viewport.min - offset - origin;
            for (_, rect) in &mut setup.rows {
                *rect = rect.translate(correction);
            }
        }
        if setup.clicked.is_some() {
            state.selected = setup.selected;
        }
        for ((column, &old), &new) in self.columns.iter().zip(&measured).zip(&setup.measured) {
            if new > old
                && column.width == super::ColumnWidth::Content
                && !state.widths.contains_key(&column.id)
            {
                child.context.request_repaint();
            }
            state.measured.insert(column.id, new);
        }
        child.clip = ui.clip.intersect(header_rect);
        nodes.open_header(child.context);
        let sort_request = header::paint(
            &mut child,
            id,
            header_rect,
            ui.clip,
            offset.x,
            &self.columns,
            &widths,
            style,
            &mut state,
            self.resizable,
        );
        let selected_row = state.selected;
        child.context.tables.insert(id, state);
        let body = ScrollArea::state_id(&child, Id::new("body"));
        let total = setup.total.unwrap_or(setup.rows.len());
        nodes.finish(ui, header_rect, rect, total, body);
        TableOutput {
            inner,
            rect,
            header_rect,
            viewport,
            widths,
            offset,
            selected_row,
            row_clicked: setup.clicked,
            sort_request,
            row_moved: setup.moved,
            rows: setup.rows,
        }
    }
}

/// Keep rectangular row/scroll clips inside the rounded frame, including custom
/// radii and zero requested padding. Existing per-cell/scroll clipping stays intact.
fn content_bounds(rect: Rect, style: TableStyle) -> Rect {
    let [tl, tr, br, bl] = style.rounding.values(rect);
    let inset = |radius: f32| {
        radius * (1.0 - std::f32::consts::FRAC_1_SQRT_2) + style.border.width.max(0.0)
    };
    Padding {
        left: style.padding.left.max(inset(tl.max(bl))),
        right: style.padding.right.max(inset(tr.max(br))),
        top: style.padding.top.max(inset(tl.max(tr))),
        bottom: style.padding.bottom.max(inset(bl.max(br))),
    }
    .inset(rect)
}
