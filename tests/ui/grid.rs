use crate::prelude::*;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::{
    Align, Column, ColumnWidth, Grid, GridOutput, GridStyle, Padding, Rect, ScrollArea, Shape,
    Window,
};

pub(crate) fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    c
}
pub(crate) fn click(c: &mut Context, rect: Rect) {
    c.move_pointer(rect.center());
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn style() -> GridStyle {
    GridStyle {
        padding: Padding::all(0.0),
        cell_padding: Padding::all(0.0),
        spacing: Vec2::new(4.0, 3.0),
        ..Default::default()
    }
}

#[test]
fn columns_resolve_fixed_content_weighted_remainder_and_overflow() {
    let columns = [
        Column::fixed("fixed", 80.0),
        Column::content("auto").min_width(20.0),
        Column::remainder("one").min_width(10.0),
        Column::new("two", ColumnWidth::Remainder(2.0)).min_width(10.0),
    ];
    assert_eq!(
        zaxis::components::columns::resolve(&columns, &[0.0, 50.0], 400.0, 10.0),
        [80.0, 50.0, 10.0 + 220.0 / 3.0, 10.0 + 440.0 / 3.0]
    );
    assert_eq!(
        zaxis::components::columns::resolve(&columns, &[0.0, 50.0], 5.0, 10.0),
        [80.0, 50.0, 10.0, 10.0]
    );
}

fn measured(c: &mut Context, calls: &mut usize) -> GridOutput<()> {
    let mut out = None;
    c.run(|c| {
        Window::new("Grid")
            .default_size(Vec2::new(500.0, 400.0))
            .show(c, |ui| {
                out = Some(
                    Grid::new("form")
                        .style(style())
                        .width(350.0)
                        .columns([
                            Column::content("label"),
                            Column::fixed("fixed", 80.0).align(Align::Center, Align::End),
                            Column::remainder("rest"),
                        ])
                        .show(ui, |grid| {
                            for (key, size) in
                                [("a", Vec2::new(45.0, 20.0)), ("b", Vec2::new(100.0, 50.0))]
                            {
                                grid.row(key, |row| {
                                    row.cell(|ui| {
                                        *calls += 1;
                                        let rect = ui.allocate_space(size);
                                        ui.paint(Shape::rect(rect, zaxis::Color::WHITE));
                                    });
                                    row.cell(|ui| {
                                        *calls += 1;
                                        let rect = ui.allocate_space(Vec2::new(20.0, 10.0));
                                        ui.paint(Shape::rect(rect, zaxis::Color::gray(123)));
                                    });
                                    row.cell(|ui| {
                                        *calls += 1;
                                        ui.horizontal(|ui| {
                                            ui.label("Label");
                                            ui.button("Action");
                                        });
                                    });
                                });
                            }
                        }),
                );
            });
    });
    out.unwrap()
}
#[test]
fn callbacks_once_shared_widths_max_height_alignment_and_idle() {
    let mut c = setup();
    let mut calls = 0;
    let first = measured(&mut c, &mut calls);
    assert_eq!(calls, 6);
    assert_eq!(first.widths, [100.0, 80.0, 162.0]);
    assert!(first.rows[1].size().y >= 50.0);
    assert_eq!(first.cells[0][1].size().x, first.cells[1][1].size().x);
    let second = measured(&mut c, &mut calls);
    assert_eq!(calls, 12);
    assert_eq!(second.widths, first.widths);
    assert!(
        !c.needs_repaint(),
        "settled layout must not redraw continuously"
    );
    // A centered 20 px allocation in an 80 px column has a 30 px offset.
    assert!(c.probe().cache.values().flat_map(|cached| &cached.paint).any(|p| matches!(p, Paint::Shape(Shape::Rect { rect, fill, .. }) if *fill == zaxis::Color::gray(123) && (rect.min.x - second.cells[0][1].min.x - 30.0).abs() < 0.01)));
}

#[test]
fn cell_paint_hits_clipping_nested_scroll_and_single_bound_mutation() {
    let mut c = setup();
    let mut value = 5.0;
    let mut calls = 0;
    let mut button = None;
    let mut output = None;
    for _ in 0..2 {
        c.run(|c| {
            Window::new("Grid").show(c, |ui| {
                output = Some(
                    Grid::new("outer")
                        .style(style())
                        .columns([
                            Column::fixed("first", 100.0),
                            Column::fixed("second", 100.0),
                        ])
                        .show(ui, |grid| {
                            grid.row("stable", |row| {
                                row.cell(|ui| {
                                    calls += 1;
                                    ui.add(zaxis::Slider::new(&mut value, 0.0..=1.0));
                                    ScrollArea::vertical()
                                        .id_source("inside")
                                        .max_height(50.0)
                                        .show(ui, |ui| {
                                            for n in 0..5 {
                                                ui.push_id(n, |ui| {
                                                    ui.button("Nested");
                                                });
                                            }
                                        });
                                });
                                row.cell(|ui| {
                                    button = Some(
                                        ui.add(
                                            zaxis::Button::new("Overflow")
                                                .min_size(Vec2::new(250.0, 30.0)),
                                        ),
                                    );
                                });
                            })
                        }),
                );
            });
        });
    }
    assert_eq!(calls, 2);
    assert_eq!(value, 1.0);
    let output = output.unwrap();
    let button = button.unwrap();
    let hit = *c
        .probe()
        .previous_hits
        .iter()
        .find(|hit| hit.id == button.id)
        .unwrap();
    assert_eq!(hit.clip, output.cells[0][1]);
    assert!(
        hit.rect.size().x <= hit.clip.size().x,
        "built-in buttons constrain their own width"
    );
    for state in c.probe().scrolling.states.values() {
        assert!(state.viewport.min.x >= output.cells[0][0].min.x);
        assert!(state.viewport.max.x <= output.cells[0][0].max.x);
    }
    assert_eq!(c.probe().placements.outstanding, 0);
    assert!(c.probe().placements.stack.is_empty());
    assert!(c.probe().scrolling.pending.is_empty());
    click(
        &mut c,
        Rect::from_min_size(hit.clip.max + Vec2::new(1.0, -15.0), Vec2::splat(1.0)),
    );
    assert!(!c.clicked(button.id));
}

#[test]
fn nested_grid_scroll_corrections_and_zero_space_have_nonnegative_rects() {
    let mut c = setup();
    c.run(|c| {
        Window::new("Tiny").show(c, |ui| {
            ScrollArea::both()
                .max_height(90.0)
                .scroll_offset(Vec2::splat(1000.0))
                .show(ui, |ui| {
                    Grid::new("outer")
                        .width(0.0)
                        .style(GridStyle {
                            padding: Padding::all(30.0),
                            cell_padding: Padding::all(20.0),
                            ..style()
                        })
                        .columns([
                            Column::remainder("a"),
                            Column::remainder("b").min_width(5.0),
                        ])
                        .show(ui, |grid| {
                            grid.row(1, |row| {
                                row.cell(|ui| {
                                    Grid::new("inner")
                                        .column(Column::remainder("c"))
                                        .show(ui, |grid| {
                                            grid.row(2, |row| row.cell(|ui| ui.label("Nested")))
                                        });
                                });
                                row.cell(|ui| {
                                    ui.label("Long content that must be clipped");
                                });
                            })
                        });
                });
        });
    });
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.rect.size().min_element() >= 0.0 && h.clip.size().min_element() >= 0.0));
    assert!(c
        .draw_data()
        .vertices
        .iter()
        .all(|v| v.position.iter().all(|n| n.is_finite())));
}

struct CountWidget<'a>(&'a mut usize);
impl zaxis::Widget for CountWidget<'_> {
    fn ui(self, ui: &mut zaxis::Ui<'_>) -> zaxis::Response {
        *self.0 += 1;
        let rect = ui.allocate_space(Vec2::new(400.0, 20.0));
        ui.paint(Shape::rect(rect, zaxis::Color::gray(124)));
        ui.button("Custom widget")
    }
}
#[test]
fn arbitrary_widget_overflow_is_clipped_and_evaluated_once() {
    let mut c = setup();
    let mut calls = 0;
    let mut output = None;
    c.run(|c| {
        Window::new("Custom").show(c, |ui| {
            output = Some(
                Grid::new("custom")
                    .style(style())
                    .column(Column::fixed("cell", 60.0))
                    .show(ui, |grid| {
                        grid.row(1, |row| row.cell(|ui| ui.add(CountWidget(&mut calls))))
                    }),
            );
        });
    });
    let out = output.unwrap();
    assert_eq!(calls, 1);
    assert_eq!(out.widths, [60.0]);
    assert_eq!(out.measured_widths, [400.0]);
    let hit = c
        .probe()
        .previous_hits
        .iter()
        .find(|hit| hit.id == out.inner.id)
        .unwrap();
    assert_eq!(hit.clip, out.cells[0][0]);
    assert!(c
        .draw_data()
        .commands
        .iter()
        .any(|command| command.clip_rect == out.cells[0][0]));
}

#[test]
fn aligned_text_edit_ime_moves_with_cell_and_nested_scroll() {
    let mut c = setup();
    let mut text = "Editable".to_owned();
    let mut field = None;
    let mut output = None;
    for pass in 0..2 {
        if pass == 1 {
            c.request_focus(field.unwrap());
        }
        c.run(|c| {
            Window::new("Editing").show(c, |ui| {
                ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                    output = Some(
                        Grid::new("form")
                            .style(style())
                            .columns([
                                Column::fixed("label", 80.0),
                                Column::fixed("value", 150.0).align(Align::Start, Align::End),
                            ])
                            .show(ui, |grid| {
                                grid.row("name", |row| {
                                    row.cell(|ui| {
                                        ui.allocate_space(Vec2::new(80.0, 100.0));
                                    });
                                    row.cell(|ui| {
                                        field = Some(ui.text_edit(&mut text).id);
                                    });
                                })
                            }),
                    );
                });
            });
        });
    }
    let out = output.unwrap();
    let ime = c
        .probe()
        .ime_area
        .expect("focused field emits IME geometry");
    assert!(ime.min.y >= out.cells[0][1].min.y + 60.0);
    assert!(ime.max.y <= out.cells[0][1].max.y);
}
