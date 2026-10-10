//! Placement: edges, deferred cells, visual scopes, scrolling and DPI.
use super::*;
use winit::dpi::PhysicalSize;
use zaxis::{Grid, ScrollArea, Window};

fn inside(viewport: Rect, rect: Rect) -> bool {
    rect.min.x >= viewport.min.x - 0.01
        && rect.min.y >= viewport.min.y - 0.01
        && rect.max.x <= viewport.max.x + 0.01
        && rect.max.y <= viewport.max.y + 0.01
}

#[test]
fn a_chain_at_the_corner_fits_the_viewport_and_opens_upward() {
    let mut c = setup();
    let mut s = Scene::new();
    s.a_pos = vec2(600.0, 460.0);
    open_chain(&mut c, &mut s, 3);
    let viewport = c.viewport();
    let a = s.a_rect.unwrap();
    assert!(
        inside(viewport, a) && a.max.y <= s.a_btn.min.y,
        "the root opens upward"
    );
    for popup in c.probe().popups {
        assert!(inside(viewport, popup.rect), "{:?}", popup.rect);
    }
    // The child fits against its own anchor inside the root, not against the root.
    let child = s.child_rect.unwrap();
    assert!(child.min.y >= s.child_btn.max.y || child.max.y <= s.child_btn.min.y);
}

#[test]
fn every_edge_keeps_every_level_on_screen() {
    for (x, y) in [
        (4.0, 4.0),
        (610.0, 4.0),
        (4.0, 466.0),
        (610.0, 466.0),
        (310.0, 240.0),
    ] {
        let mut c = setup();
        let mut s = Scene::new();
        s.a_pos = vec2(x, y);
        open_chain(&mut c, &mut s, 3);
        let viewport = c.viewport();
        assert_eq!(c.probe().popups.len(), 3, "at {x},{y}");
        for popup in c.probe().popups {
            assert!(inside(viewport, popup.rect), "at {x},{y}: {:?}", popup.rect);
        }
    }
}

#[test]
fn the_branch_follows_its_triggers_when_a_window_moves() {
    let mut c = setup();
    let mut offset = vec2(0.0, 0.0);
    let mut open = (true, true);
    let pass = |c: &mut Context, offset: Vec2, open: &mut (bool, bool)| {
        c.run(|c| {
            Window::new("host")
                .default_position(vec2(40.0, 40.0))
                .offset(offset)
                .default_size(vec2(300.0, 200.0))
                .show(c, |ui| {
                    let trigger = ui.add(Button::new("trigger"));
                    Popup::new("root", trigger.rect)
                        .size(vec2(160.0, 100.0))
                        .show(ui, &mut open.0, |ui| {
                            let inner = ui.add(Button::new("inner"));
                            Popup::new("leaf", inner.rect).size(vec2(100.0, 50.0)).show(
                                ui,
                                &mut open.1,
                                |ui| {
                                    ui.label("leaf");
                                },
                            );
                        });
                });
        });
    };
    for _ in 0..3 {
        pass(&mut c, offset, &mut open);
    }
    assert_eq!(c.probe().popups.len(), 2);
    let popups = c.probe().popups;
    assert!(popups[1].anchor.min.y >= popups[0].rect.min.y);
    let before = (popups[0].anchor, popups[1].anchor);
    offset = vec2(30.0, 20.0);
    for _ in 0..3 {
        pass(&mut c, offset, &mut open);
    }
    let popups = c.probe().popups;
    assert_eq!(popups.len(), 2, "moving the window keeps the branch open");
    assert_eq!(popups[0].anchor, before.0.translate(offset));
    assert_eq!(popups[1].anchor, before.1.translate(offset));
    assert!(inside(c.viewport(), popups[1].rect));
}

#[test]
fn a_popup_in_a_scroll_area_and_a_grid_cell_follows_its_anchor() {
    let mut c = setup();
    let mut open = (true, true);
    let mut row_rect = Rect::default();
    for _ in 0..3 {
        c.run(|c| {
            Window::new("host")
                .default_size(vec2(400.0, 300.0))
                .show(c, |ui| {
                    ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                        ui.add_space(30.0);
                        Grid::new("g")
                            .columns([zaxis::Column::content("a"), zaxis::Column::remainder("b")])
                            .show(ui, |grid| {
                                grid.row("r", |row| {
                                    row.cell(|ui| {
                                        ui.label("name");
                                    });
                                    row.cell(|ui| {
                                        let b = ui.add(Button::new("edit"));
                                        row_rect = b.rect;
                                        Popup::new("p", b.rect).size(vec2(140.0, 90.0)).show(
                                            ui,
                                            &mut open.0,
                                            |ui| {
                                                let inner = ui.add(Button::new("more"));
                                                Popup::new("q", inner.rect)
                                                    .size(vec2(100.0, 40.0))
                                                    .show(ui, &mut open.1, |ui| {
                                                        ui.label("q");
                                                    });
                                            },
                                        );
                                    });
                                });
                            });
                    });
                });
        });
    }
    let popups = c.probe().popups;
    assert_eq!(popups.len(), 2);
    assert!(!popups[0].anchor.intersect(row_rect).is_empty());
    let viewport = c.viewport();
    assert!(popups.iter().all(|p| inside(viewport, p.rect)));
    // Hit regions of the child sit inside the child's rectangle, in the child's layer.
    let leaf = popups[1].id;
    let hits: Vec<_> = c
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.window == leaf)
        .copied()
        .collect();
    assert!(!hits.is_empty());
    assert!(hits
        .iter()
        .filter(|h| h.action != HitAction::Block)
        .all(|h| popups[1].rect.contains(h.rect.center())));
}

#[test]
fn a_visual_scale_moves_and_scales_the_branch_with_the_scene() {
    let mut c = setup();
    let mut open = (true, true);
    let transform = zaxis::Transform::around(Vec2::ZERO, 1.5, vec2(40.0, 30.0));
    for _ in 0..3 {
        c.run(|c| {
            Root::new().show(c, |ui| {
                ui.visual("v", transform, 1.0, |ui| {
                    let b = ui.add(Button::new("trigger"));
                    Popup::new("p", b.rect)
                        .size(vec2(120.0, 80.0))
                        .show(ui, &mut open.0, |ui| {
                            let inner = ui.add(Button::new("more"));
                            Popup::new("q", inner.rect).size(vec2(80.0, 40.0)).show(
                                ui,
                                &mut open.1,
                                |ui| {
                                    ui.label("q");
                                },
                            );
                        });
                });
            });
        });
    }
    let popups = c.probe().popups;
    assert_eq!(popups.len(), 2);
    let viewport = c.viewport();
    assert!(popups.iter().all(|p| inside(viewport, p.rect)));
    assert!(
        !popups[1].anchor.intersect(popups[0].rect).is_empty(),
        "the child hangs off the parent"
    );
}

#[test]
fn dpi_changes_keep_the_branch_open_and_on_screen() {
    let mut c = setup();
    let mut s = Scene::new();
    open_chain(&mut c, &mut s, 3);
    for scale in [1.5_f64, 2.0, 1.0] {
        c.set_viewport(
            PhysicalSize::new((700.0 * scale) as u32, (500.0 * scale) as u32),
            scale,
        );
        settle(&mut c, &mut s);
        assert_eq!(c.probe().popups.len(), 3, "at {scale}");
        let viewport = c.viewport();
        assert!(c.probe().popups.iter().all(|p| inside(viewport, p.rect)));
    }
}
