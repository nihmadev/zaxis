use crate::grid::{click, setup};
use crate::prelude::*;
use winit::event::ElementState;
use zaxis::{Column, Rect, SortDirection, Table, TableOutput, Window};

fn columns() -> [Column; 3] {
    [
        Column::fixed("name", 120.0)
            .min_width(60.0)
            .title("Name")
            .sortable(true),
        Column::content("status").min_width(60.0).title("Status"),
        Column::remainder("action").min_width(80.0).title("Action"),
    ]
}
fn table(c: &mut Context, offset: Option<Vec2>, order: &[usize]) -> TableOutput<()> {
    let mut output = None;
    c.run(|c| {
        Window::new("Table")
            .default_size(Vec2::new(550.0, 400.0))
            .show(c, |ui| {
                let mut table = Table::new("items")
                    .columns(columns())
                    .max_height(160.0)
                    .separators(true);
                if let Some(offset) = offset {
                    table = table.scroll_offset(offset);
                }
                output = Some(table.show(ui, |body| {
                    for &n in order {
                        body.row(n, |row| {
                            row.cell(|ui| {
                                ui.label(format!("Item {n}"));
                            });
                            row.cell(|ui| {
                                ui.label("Ready");
                            });
                            row.cell(|ui| {
                                ui.button("Open");
                            });
                        });
                    }
                }));
            });
    });
    output.unwrap()
}

#[test]
fn matching_header_body_widths_sort_requests_selection_and_reordering() {
    let mut c = setup();
    let first = table(&mut c, None, &[10, 20, 30]);
    let header = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| {
            matches!(h.action, HitAction::Activate) && h.rect.min.y == first.header_rect.min.y
        })
        .unwrap()
        .rect;
    assert_eq!(header.size().x, first.widths[0]);
    click(&mut c, header);
    let sorted = table(&mut c, None, &[10, 20, 30]);
    assert_eq!(
        sorted.sort_request,
        Some(zaxis::SortRequest {
            column: Id::new("name"),
            direction: SortDirection::Ascending
        })
    );
    click(&mut c, header);
    let sorted = table(&mut c, None, &[10, 20, 30]);
    assert_eq!(
        sorted.sort_request.unwrap().direction,
        SortDirection::Descending
    );
    let row = sorted.rows[1].1;
    click(
        &mut c,
        Rect::from_min_size(row.min, Vec2::new(60.0, row.size().y)),
    );
    let selected = table(&mut c, None, &[10, 20, 30]);
    assert_eq!(selected.row_clicked, Some(Id::new(20usize)));
    let reordered = table(&mut c, None, &[30, 10, 20]);
    assert_eq!(reordered.selected_row, Some(Id::new(20usize)));
    assert!(reordered.sort_request.is_none());
    for (_, rect) in &reordered.rows {
        assert_eq!(rect.size().x, reordered.widths.iter().sum::<f32>());
    }
}

#[test]
fn resize_capture_minimum_and_shared_widths_after_scrolling() {
    let mut c = setup();
    let order: Vec<_> = (0..30).collect();
    let first = table(&mut c, None, &order);
    let handle = c.probe().previous_hits.iter().find(|h| matches!(h.action, HitAction::ColumnResize { column, .. } if column == Id::new("name"))).unwrap().rect;
    c.move_pointer(handle.center());
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::ColResize);
    c.primary_button(ElementState::Pressed);
    c.move_pointer(handle.center() + Vec2::new(1000.0, 100.0));
    let wide = table(&mut c, None, &order);
    assert_eq!(wide.widths[0], first.widths[0] + 1000.0);
    assert!(
        c.probe().capture.is_some(),
        "resize capture survives a handle moving offscreen"
    );
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::ColResize);
    c.move_pointer(handle.center() + Vec2::new(75.0, 100.0));
    let resized = table(&mut c, None, &order);
    assert_eq!(resized.widths[0], first.widths[0] + 75.0);
    assert_eq!(resized.header_rect, first.header_rect);
    c.primary_button(ElementState::Released);
    assert_eq!(c.cursor_icon(), winit::window::CursorIcon::Default);
    let scrolled = table(&mut c, Some(Vec2::new(0.0, 80.0)), &order);
    assert_eq!(scrolled.header_rect, first.header_rect);
    assert_eq!(scrolled.rows[0].1.min.y, first.rows[0].1.min.y - 80.0);
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.action == HitAction::Activate)
        .all(|h| h.clip.min.y >= scrolled.viewport.min.y || h.clip == scrolled.header_rect));
    let handle = c.probe().previous_hits.iter().find(|h| matches!(h.action, HitAction::ColumnResize { column, .. } if column == Id::new("name"))).unwrap().rect;
    c.move_pointer(handle.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(handle.center() - Vec2::new(1000.0, 0.0));
    c.primary_button(ElementState::Released);
    assert_eq!(table(&mut c, None, &order).widths[0], 60.0);
}

fn virtual_table(
    c: &mut Context,
    total: usize,
    offset: Vec2,
    reverse: bool,
) -> (TableOutput<std::ops::Range<usize>>, usize, Vec<(usize, Id)>) {
    let mut output = None;
    let mut calls = 0;
    let mut ids = Vec::new();
    c.run(|c| {
        Window::new("Virtual").show(c, |ui| {
            output = Some(
                Table::new("items")
                    .columns(columns())
                    .max_height(160.0)
                    .scroll_offset(offset)
                    .show_rows(ui, 28.0, total, |body, index| {
                        calls += 1;
                        let key = if reverse { total - index - 1 } else { index };
                        body.row(key, |row| {
                            row.cell(|ui| {
                                ids.push((key, ui.button(format!("Item {key}")).id));
                            });
                            row.cell(|ui| {
                                ui.label("Ready");
                            });
                            row.cell(|ui| {
                                ui.horizontal(|ui| {
                                    ui.button("A");
                                    ui.button("B");
                                });
                            });
                        });
                    }),
            );
        });
    });
    (output.unwrap(), calls, ids)
}
#[test]
fn long_fixed_list_builds_only_visible_rows_clamps_and_keeps_ids_across_sort() {
    let mut c = setup();
    let (first, calls, ids) = virtual_table(&mut c, 10_000, Vec2::ZERO, false);
    assert!(calls <= 6);
    assert_eq!(calls, first.inner.len());
    assert_eq!(first.rows.len(), calls);
    let button_id = ids.iter().find(|(key, _)| *key == 0).unwrap().1;
    let (end, calls, sorted_ids) = virtual_table(&mut c, 10_000, Vec2::new(0.0, 1.0e9), true);
    assert!(calls <= 6);
    assert_eq!(end.inner.end, 10_000);
    assert_eq!(
        sorted_ids.iter().find(|(key, _)| *key == 0).unwrap().1,
        button_id
    );
    let (short, _, _) = virtual_table(&mut c, 2, end.offset, false);
    assert_eq!(short.offset.y, 0.0);
    assert_eq!(short.inner, 0..2);
    assert!(
        c.probe().grids.len() <= 6,
        "virtual grid layout cache only retains live rows"
    );
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .all(|h| !h.rect.intersect(h.clip).is_empty()));
    assert!(c.probe().scrolling.pending.is_empty());
}

#[test]
fn horizontal_overflow_moves_header_with_body_and_nested_clipping() {
    let mut c = setup();
    let mut output = None;
    c.run(|c| {
        Window::new("Nested").show(c, |ui| {
            zaxis::ScrollArea::vertical()
                .max_height(110.0)
                .show(ui, |ui| {
                    output = Some(
                        Table::new("wide")
                            .width(180.0)
                            .max_height(180.0)
                            .columns([
                                Column::fixed("a", 180.0).title("A").sortable(true),
                                Column::fixed("b", 180.0).title("B"),
                            ])
                            .scroll_offset(Vec2::new(60.0, 0.0))
                            .show(ui, |body| {
                                for i in 0..20 {
                                    body.row(i, |row| {
                                        row.cell(|ui| {
                                            ui.label("A");
                                        });
                                        row.cell(|ui| {
                                            ui.button("B");
                                        });
                                    });
                                }
                            }),
                    );
                });
        });
    });
    let out = output.unwrap();
    assert_eq!(out.widths, [180.0, 180.0]);
    assert_eq!(out.offset.x, 60.0);
    let header = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.rect.min.y == out.header_rect.min.y && h.action == HitAction::Activate)
        .unwrap();
    assert_eq!(header.rect.min.x, out.header_rect.min.x - 60.0);
    assert_eq!(out.rows[0].1.min.x, header.rect.min.x);
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| matches!(
            h.action,
            HitAction::Activate | HitAction::ColumnResize { .. }
        ))
        .all(|h| h.clip.max.y <= out.rect.min.y + 110.0));
}

#[test]
fn rounded_surfaces_are_default_and_custom_radii_keep_content_inside_frame() {
    let mut c = setup();
    let out = table(&mut c, None, &[1, 2, 3]);
    assert!(c
        .probe()
        .cache
        .values()
        .flat_map(|cached| &cached.paint)
        .any(|paint| {
            matches!(paint, Paint::Shape(zaxis::Shape::Rect { rect, rounding, border, .. })
            if *rect == out.rect && *rounding == c.style().table.rounding && border.width > 0.0)
        }));
    assert!(c
        .probe()
        .cache
        .values()
        .flat_map(|cached| &cached.paint)
        .any(|paint| {
            matches!(paint, Paint::Shape(zaxis::Shape::Rect { rect, rounding, .. })
            if *rect == out.header_rect && rounding.top_left == c.style().table.rounding.top_left
                && rounding.top_right == c.style().table.rounding.top_right)
        }));
    let mut custom = None;
    c.run(|c| {
        Window::new("Round").show(c, |ui| {
            custom = Some(
                Table::new("round")
                    .columns(columns())
                    .corner_radius(40.0)
                    .padding(zaxis::Padding::all(0.0))
                    .show(ui, |body| {
                        body.row(1, |row| {
                            row.cell(|ui| {
                                ui.label("Safe content");
                            });
                        });
                    }),
            );
        });
    });
    let custom = custom.unwrap();
    let corner = custom.header_rect.min;
    let center = custom.rect.min + Vec2::splat(40.0);
    assert!(
        (corner - center).length() < 40.0,
        "content clip remains inside the corner arc"
    );
}
