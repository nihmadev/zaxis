use super::{Column, Id, Rect, SortDirection, SortRequest, TableState, TableStyle, Ui};
use crate::{
    context::{HitAction, HitRegion, Paint},
    Shape, Vec2,
};

pub(super) fn resize(
    ui: &mut Ui<'_>,
    table: Id,
    columns: &[Column],
    state: &mut TableState,
    enabled: bool,
) {
    for column in columns {
        if let Some(width) = ui.context.take_column_resize(table, column.id) {
            if enabled && column.resizable {
                state.widths.insert(column.id, width.max(column.minimum));
            }
        }
    }
}
pub(super) fn paint(
    ui: &mut Ui<'_>,
    table: Id,
    rect: Rect,
    parent_clip: Rect,
    offset: f32,
    columns: &[Column],
    widths: &[f32],
    style: TableStyle,
    state: &mut TableState,
    resizable: bool,
) -> Option<SortRequest> {
    let clip = parent_clip.intersect(rect);
    state.resize_columns = columns
        .iter()
        .filter(|column| resizable && column.resizable && ui.enabled)
        .map(|column| column.id)
        .collect();
    ui.context.paint(
        table.with("header-fill"),
        ui.window,
        clip,
        vec![Paint::Shape(
            Shape::rect(rect, style.header_fill)
                .corner_radius(crate::CornerRadius {
                    top_left: style.rounding.top_left,
                    top_right: style.rounding.top_right,
                    bottom_left: 0.0,
                    bottom_right: 0.0,
                })
                .into(),
        )],
    );
    let mut x = rect.min.x - offset;
    let mut request = None;
    for (column, &width) in columns.iter().zip(widths) {
        let bounds = Rect::from_min_size(Vec2::new(x, rect.min.y), Vec2::new(width, rect.size().y));
        let id = table.with(("header", column.id));
        let response = ui.response(id, bounds, column.sortable);
        if response.clicked {
            let direction = if state
                .sort
                .is_some_and(|s| s.column == column.id && s.direction == SortDirection::Ascending)
            {
                SortDirection::Descending
            } else {
                SortDirection::Ascending
            };
            let sort = SortRequest {
                column: column.id,
                direction,
            };
            state.sort = Some(sort);
            request = Some(sort);
            ui.context.request_repaint();
        }
        let suffix = state
            .sort
            .filter(|s| s.column == column.id)
            .map_or("", |s| match s.direction {
                SortDirection::Ascending => " ↑",
                SortDirection::Descending => " ↓",
            });
        let text = format!("{}{suffix}", column.title);
        let size = super::super::font_size(style.font_size);
        let measured = ui.context.measure_text(&text, size, f32::INFINITY);
        let padded = style.cell_padding.inset(bounds);
        let position = padded.min
            + Vec2::new(
                column.horizontal.offset(padded.size().x - measured.x),
                (padded.size().y - measured.y).max(0.0) * 0.5,
            );
        ui.context.paint(
            id.with("text"),
            ui.window,
            clip.intersect(padded),
            vec![Paint::Text {
                text,
                position,
                size,
                wrap_width: f32::INFINITY,
                color: style.text_color,
            }],
        );
        if column.sortable && ui.enabled {
            ui.context.register_hit(HitRegion {
                id,
                window: ui.window,
                rect: bounds,
                clip,
                action: HitAction::Activate,
            });
        }
        if resizable && column.resizable && ui.enabled {
            let handle = Rect::from_min_size(
                Vec2::new(
                    bounds.max.x - style.resize_handle_width.max(0.0) * 0.5,
                    bounds.min.y,
                ),
                Vec2::new(style.resize_handle_width.max(0.0), bounds.size().y),
            );
            let handle_id = table.with(("resize", column.id));
            ui.context.register_hit(HitRegion {
                id: handle_id,
                window: ui.window,
                rect: handle,
                clip,
                action: HitAction::ColumnResize {
                    table,
                    column: column.id,
                },
            });
            if ui.context.hovered(handle_id, ui.window, handle, clip)
                || ui.context.active(handle_id)
            {
                ui.context.paint(
                    handle_id.with("line"),
                    ui.window,
                    clip,
                    vec![Paint::Shape(
                        Shape::rect(
                            Rect::from_min_size(
                                Vec2::new(
                                    bounds.max.x - style.separator_width.max(1.0),
                                    bounds.min.y,
                                ),
                                Vec2::new(style.separator_width.max(1.0), bounds.size().y),
                            ),
                            style.text_color,
                        )
                        .into(),
                    )],
                );
            }
        }
        state
            .widths
            .entry(column.id)
            .and_modify(|stored| *stored = width);
        x += width;
    }
    // Resolved widths are needed when capture begins, including flexible columns.
    state.current_widths = columns
        .iter()
        .zip(widths)
        .map(|(c, &w)| (c.id, w))
        .collect();
    request
}
