use std::{sync::mpsc, time::Duration};
use zaxis::{
    vec2, winit, Column, Context, Grid, Image, ImageFit, ImageSource, ScrollArea, Table, Transform,
    Window,
};

#[test]
fn images_use_grid_table_clips_and_visual_physical_resolution() {
    static SVG:&[u8]=br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><path d="M0 0H20V10H0Z" fill="red"/></svg>"#;
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(1600, 1200), 2.0);
    let (tx, rx) = mpsc::channel();
    c.set_image_waker(move || {
        let _ = tx.send(());
    });
    let h = c.load_image(SVG).unwrap();
    let mut logical = None;
    let mut build = |c: &mut Context| {
        c.run(|c| {
            Window::new("containers")
                .default_size(vec2(700.0, 500.0))
                .show(c, |ui| {
                    Grid::new("grid")
                        .columns([Column::fixed("image", 100.0)])
                        .show(ui, |grid| {
                            grid.row(0, |row| {
                                row.cell(|ui| {
                                    logical = Some(
                                        ui.add(
                                            Image::new(h)
                                                .size(vec2(40.0, 20.0))
                                                .fit(ImageFit::Stretch),
                                        )
                                        .rect
                                        .size(),
                                    );
                                });
                            });
                        });
                    Table::new("table")
                        .columns([Column::fixed("image", 100.0)])
                        .max_height(60.0)
                        .show(ui, |table| {
                            table.row(0, |row| {
                                row.cell(|ui| {
                                    ui.add(Image::new(h).size(vec2(20.0, 10.0)));
                                });
                            });
                        });
                    ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                        ui.visual(
                            "scaled",
                            Transform::around(ui.clip_rect().min, 2.0, vec2(0.0, 0.0)),
                            0.5,
                            |ui| {
                                ui.add(
                                    Image::new(h)
                                        .size(vec2(40.0, 20.0))
                                        .fit(ImageFit::Stretch)
                                        .opacity(0.5),
                                );
                            },
                        );
                    });
                });
        });
    };
    build(&mut c);
    rx.recv_timeout(Duration::from_secs(10)).unwrap();
    build(&mut c);
    assert!(c.image_state(h).is_ready());
    assert_eq!(logical, Some(vec2(40.0, 20.0)));
    let data = c.draw_data();
    let id = *data.texture_options.keys().next().unwrap();
    assert_eq!(
        data.textures.iter().find(|t| t.id == id).unwrap().size,
        [160, 80]
    );
    assert!(data.commands.iter().filter(|cmd| cmd.texture == id).count() >= 3);
    assert!(data
        .commands
        .iter()
        .filter(|cmd| cmd.texture == id)
        .all(|cmd| !cmd.clip_rect.is_empty()));
    assert!(data
        .vertices
        .iter()
        .any(|v| (v.color[3] - 0.25).abs() < 0.001));
}

#[test]
fn completely_clipped_image_does_not_start_worker_or_repaint_loop() {
    let mut c = Context::new();
    c.set_viewport(winit::dpi::PhysicalSize::new(800, 600), 1.0);
    let source = ImageSource::rgba([32, 16], vec![255; 32 * 16 * 4]);
    c.run(|c| {
        Window::new("clipped")
            .default_size(vec2(200.0, 100.0))
            .show(c, |ui| {
                ui.allocate_space(vec2(0.0, 1000.0));
                ui.image(&source);
            });
    });
    assert_eq!(c.image_metrics().pending_jobs, 0);
    assert_eq!(c.image_metrics().decodes, 0);
    assert!(c.next_repaint().is_none());
}
