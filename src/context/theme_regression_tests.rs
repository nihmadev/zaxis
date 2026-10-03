use super::*;
use crate::*;
use winit::dpi::PhysicalSize;

#[test]
fn background_blur_leaves_controls_opaque_and_scroll_hints_soft() {
    for mut theme in [Theme::dark(), Theme::light(), Theme::high_contrast()] {
        theme.metrics.blur = 12.0;
        theme.overrides.motion = Some(MotionStyle {
            reduced_motion: true,
            ..Default::default()
        });
        let mut c = Context::new();
        c.set_viewport(PhysicalSize::new(1100, 1200), 1.0);
        c.set_theme(theme);
        let mut checked = true;
        let mut value = 0.5;
        let mut number = 42.0;
        let mut text = String::new();
        let mut color = Color::WHITE;
        let mut selected = Some(0);
        let options = [ComboBoxOption::new(0, 0, "Option")];
        let mut button = Id::new(0);
        c.run(|c| {
            Root::new().show(c, |ui| {
                button = ui.button("Button").id;
                ui.checkbox(&mut checked, "Checked");
                ui.slider(&mut value, 0.0..=1.0);
                ui.add(TextEdit::new(&mut text));
                ui.number_input(&mut number);
                ui.add(ComboBox::new(&mut selected, &options));
                ui.add(ColorPicker::new(&mut color, "Color").default_open(true));
                ui.add(Progress::new(ProgressState::Determinate(value)));
                Grid::new("grid")
                    .column(Column::remainder("cell"))
                    .show(ui, |grid| {
                        grid.row(0, |row| {
                            row.cell(|ui| {
                                ui.button("Cell");
                            })
                        });
                    });
                Table::new("table")
                    .column(Column::remainder("cell"))
                    .max_height(100.0)
                    .show(ui, |body| {
                        for i in 0..10 {
                            body.row(i, |row| {
                                row.cell(|ui| {
                                    ui.label("Row");
                                })
                            });
                        }
                    });
                ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                    for i in 0..10 {
                        ui.push_id(i, |ui| ui.button("Item"));
                    }
                });
            });
        });
        assert_eq!(
            c.draw_data()
                .commands
                .iter()
                .filter_map(|c| c.blur)
                .collect::<Vec<_>>(),
            [12.0],
            "only Root filters the background"
        );
        let body = &c.cache[&button.with("body")].paint;
        assert!(
            body.iter().any(|p| matches!(p,
                Paint::Shape(Shape::Rect { fill, .. }) if *fill == c.style.button_fill
            )),
            "background blur must not alter the button fill or alpha"
        );
        let hints = c
            .cache
            .values()
            .flat_map(|entry| &entry.paint)
            .filter_map(|p| {
                if let Paint::ScrollHint { color, .. } = p {
                    Some(*color)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert!(!hints.is_empty());
        assert!(hints.iter().all(|color| *color == Color::rgba(0, 0, 0, 16)));
    }
}

#[test]
fn control_blur_is_opt_in_and_explicit_zero_cancels_it() {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(900, 600), 1.0);
    let mut theme = Theme::dark();
    theme.metrics.blur = 12.0;
    theme.overrides.button.surface.idle.blur = Some(3.0);
    c.set_theme(theme);
    c.run(|c| {
        Root::new().show(c, |ui| {
            ui.button("Explicit theme override");
            ui.add(Button::new("Explicit zero").blur(0.0));
        })
    });
    assert_eq!(
        c.draw_data()
            .commands
            .iter()
            .filter_map(|c| c.blur)
            .collect::<Vec<_>>(),
        [12.0, 3.0]
    );
}
