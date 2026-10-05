use crate::prelude::*;
use winit::dpi::PhysicalSize;
use zaxis::{Button, Column, FontWeight, Grid, GridStyle, Root, Table, Vec2};

#[test]
fn buttons_fit_fixed_height_cells_and_center_their_captions() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(600, 400), 1.25);
    let mut buttons = Vec::new();
    c.run(|c| {
        Root::new().show(c, |ui| {
            Table::new("compact")
                .max_height(180.0)
                .columns([Column::remainder("action")])
                .show_rows(ui, 28.0, 3, |body, i| {
                    body.row(i, |row| {
                        row.cell(|ui| {
                            buttons.push(ui.button(format!("Build module {i}")));
                        });
                    })
                });
            Grid::new("large")
                .column(Column::remainder("action"))
                .style(GridStyle {
                    row_min_height: 70.0,
                    ..GridStyle::default()
                })
                .show(ui, |grid| {
                    grid.row(0, |row| {
                        row.cell(|ui| {
                            buttons.push(
                                ui.add(Button::new("Centered").min_size(Vec2::new(200.0, 50.0))),
                            );
                        })
                    })
                });
        })
    });
    for (i, button) in buttons.into_iter().enumerate() {
        let body_clip = c
            .probe()
            .elements
            .iter()
            .find(|e| e.id == button.id.with("body"))
            .unwrap()
            .clip;
        let caption = c.probe().cache[&button.id.with("caption")].paint.clone();
        let Paint::Text {
            text,
            position,
            size,
            ..
        } = &caption[0]
        else {
            panic!("caption")
        };
        let text_size = c
            .probe_mut()
            .text
            .measure(text, *size, FontWeight::REGULAR, f32::INFINITY);
        let offset = c
            .probe_mut()
            .text
            .centered_line_offset(text, *size, FontWeight::REGULAR);
        let bounds = c.visual_rect(button.id, button.rect);
        assert!((position.x + text_size.x * 0.5 - bounds.center().x).abs() < 0.01);
        assert!((position.y + text_size.y * 0.5 - offset - bounds.center().y).abs() < 0.01);
        if i < 3 {
            assert!(bounds.size().y <= 20.0);
        }
        assert_eq!(body_clip.intersect(bounds), bounds);
        let clip = c
            .probe()
            .elements
            .iter()
            .find(|e| e.id == button.id.with("caption"))
            .unwrap()
            .clip;
        assert!(clip.size().y >= text_size.y.min(bounds.size().y) - 0.01);
    }
}
