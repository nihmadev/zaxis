use super::*;
use zaxis::{Column, Grid, PanZoom, PanZoomState, ScrollArea, Transform, Window};

fn tags(ui: &mut Ui<'_>, count: usize) -> Vec<Id> {
    (0..count)
        .map(|i| {
            let key: &'static str = Box::leak(format!("t{i}").into_boxed_str());
            block(ui, key, 70.0, 20.0).id
        })
        .collect()
}

#[test]
fn inside_a_scroll_area_items_are_reached_where_they_are_shown_after_scrolling() {
    let mut c = setup();
    let pass = |c: &mut Context, offset: Option<Vec2>| {
        let mut out = None;
        root(c, |ui| {
            let mut area = ScrollArea::vertical()
                .id_source("s")
                .max_height(120.0)
                .max_width(300.0);
            if let Some(offset) = offset {
                area = area.scroll_offset(offset);
            }
            area.show(ui, |ui| {
                out = Some(Wrap::new("w").gap(6.0).show(ui, |ui| tags(ui, 24)));
            });
        });
        out.unwrap()
    };
    let first = pass(&mut c, None);
    let lines = first.lines.len();
    assert!(lines > 4, "the content is taller than the viewport");
    // The width does not depend on the scroll bar: the layout is stable pass to pass.
    for _ in 0..4 {
        assert_eq!(pass(&mut c, None).lines.len(), lines);
    }
    let shown = pass(&mut c, Some(vec2(0.0, 60.0)));
    // The first line is above the viewport after scrolling: nothing there takes input.
    let first = shown.inner[0];
    assert!(c
        .probe()
        .previous_hits
        .iter()
        .all(|h| h.id != first || h.rect.intersect(h.clip).is_empty()));
    // An item in view after scrolling takes the click at its displayed position.
    let visible = shown
        .inner
        .iter()
        .copied()
        .find(|&id| {
            c.probe()
                .previous_hits
                .iter()
                .any(|h| h.id == id && h.rect.min.y >= 10.0 && h.rect.max.y <= 110.0)
        })
        .expect("an item is in view");
    let target = placed(&c, visible).center();
    c.move_pointer(target);
    let top = c.hit_test(target).expect("something is under the pointer");
    assert_eq!(top.id, visible);
}

#[test]
fn wrap_in_a_grid_cell_wraps_at_the_cell_width() {
    let mut c = setup();
    let mut out = None;
    root(&mut c, |ui| {
        Grid::new("g")
            .columns([Column::fixed("a", 100.0), Column::fixed("b", 200.0)])
            .show(ui, |grid| {
                grid.row("r", |row| {
                    row.cell(|ui| {
                        block(ui, "label", 40.0, 10.0);
                    });
                    row.cell(|ui| {
                        out = Some(Wrap::new("w").gap(4.0).show(ui, |ui| tags(ui, 5)));
                    });
                });
            });
    });
    let out = out.unwrap();
    // The cell has 200 minus padding for 70-wide items: two per line.
    assert!(
        out.lines.len() >= 2 && out.lines.len() <= 3,
        "{}",
        out.lines.len()
    );
    assert!(out.rect.size().x <= 200.0 + 0.01);
    let rows = placed(&c, out.inner[0]);
    assert!(rows.min.x >= 100.0, "the second column starts at x = 100");
}

#[test]
fn under_a_visual_transform_hits_and_paint_follow_the_displayed_items() {
    let mut c = setup();
    let transform = Transform {
        scale: 1.5,
        translation: vec2(40.0, 30.0),
        angle: 0.0,
    };
    let out = root(&mut c, |ui| {
        ui.visual("v", transform, 1.0, |ui| {
            Wrap::new("w")
                .width(200.0)
                .gap(10.0)
                .show(ui, |ui| tags(ui, 6))
        })
    });
    let out2 = root(&mut c, |ui| {
        ui.visual("v", transform, 1.0, |ui| {
            Wrap::new("w")
                .width(200.0)
                .gap(10.0)
                .show(ui, |ui| tags(ui, 6))
        })
    });
    assert_eq!(out.items, out2.items);
    // Item 3 is on the second line: 200 holds two 70-wide items (150), not three.
    let layout = out2.items[3];
    assert_eq!(layout.min, vec2(80.0, 30.0));
    let shown = transform.rect(layout);
    near(placed(&c, out2.inner[3]), shown);
    let p = shown.center();
    c.move_pointer(p);
    assert_eq!(c.hit_test(p).map(|h| h.id), Some(out2.inner[3]));
    // Not at the place the item was built.
    assert_ne!(
        c.hit_test(transform.rect(layout).min - vec2(5.0, 5.0))
            .map(|h| h.id),
        Some(out2.inner[3])
    );
}

#[test]
fn a_pan_zoom_scales_a_wrap_once() {
    let mut c = setup();
    let mut camera = PanZoomState {
        scale: 2.0,
        translation: vec2(5.0, 7.0),
    };
    let mut out = None;
    let mut pass = |c: &mut Context| {
        root(c, |ui| {
            PanZoom::new("pz", vec2(500.0, 400.0)).show(ui, &mut camera, |ui, _| {
                ui.at("tags", rect(10.0, 10.0, 160.0, 300.0), |ui| {
                    out = Some(Wrap::new("w").gap(5.0).show(ui, |ui| tags(ui, 4)));
                });
            });
        })
    };
    pass(&mut c);
    pass(&mut c);
    let out = out.unwrap();
    assert_eq!(
        out.lines.len(),
        2,
        "160 holds two 70-wide items in content units"
    );
    // Content (10,10) is shown at translation + 2 x content, below the viewport origin.
    let shown = placed(&c, out.inner[3]);
    assert_eq!(shown.size(), vec2(140.0, 40.0), "scaled once");
    assert!(shown.min.x > 100.0);
    let hit = c.hit_test(shown.center()).map(|h| h.id);
    assert_eq!(hit, Some(out.inner[3]));
}

#[test]
fn the_layout_is_identical_at_every_scale_factor() {
    let reference = {
        let mut c = setup_at(1.0);
        wrap_blocks(&mut c, 170.0, vec2(8.0, 6.0), &[(60.0, 14.0); 5]).1
    };
    for dpi in [1.25, 1.5, 2.0, 3.0] {
        let mut c = setup_at(dpi);
        let (_, rects) = wrap_blocks(&mut c, 170.0, vec2(8.0, 6.0), &[(60.0, 14.0); 5]);
        for (a, b) in rects.iter().zip(&reference) {
            near(*a, *b);
        }
        // The pointer is physical pixels: input reaches the same logical item.
        let target = rects[4].center() * dpi as f32;
        c.move_pointer(target / dpi as f32);
        assert!(c.hit_test(rects[4].center()).is_some());
    }
}

#[test]
fn wrapped_content_moves_with_its_window() {
    let mut c = setup();
    let mut rects = Vec::new();
    for offset in [vec2(0.0, 0.0), vec2(120.0, 80.0)] {
        let mut ids = Vec::new();
        c.run(|c| {
            Window::new("host")
                .default_size(vec2(260.0, 220.0))
                .offset(offset)
                .show(c, |ui| {
                    ids = Wrap::new("w").gap(6.0).show(ui, |ui| tags(ui, 6)).inner;
                });
        });
        rects.push(ids.iter().map(|&id| placed(&c, id)).collect::<Vec<_>>());
    }
    let (a, b) = (&rects[0], &rects[1]);
    let moved = b[0].min - a[0].min;
    assert_ne!(moved, Vec2::ZERO);
    for (x, y) in a.iter().zip(b) {
        near(Rect::from_min_size(x.min + moved, x.size()), *y);
    }
}

#[test]
fn a_scroll_target_reveals_a_wrapped_item_from_inside_it_or_from_the_report() {
    for inside in [true, false] {
        let mut c = setup();
        let mut target = None;
        let pass = |c: &mut Context, reveal: bool, target: &mut Option<Rect>| {
            let mut ids = Vec::new();
            root(c, |ui| {
                ScrollArea::vertical()
                    .id_source("s")
                    .max_height(100.0)
                    .max_width(220.0)
                    .show(ui, |ui| {
                        let out = Wrap::new("w").gap(6.0).show(ui, |ui| {
                            for i in 0..30 {
                                ui.item(i, |ui| {
                                    let b = block(ui, "tag", 70.0, 20.0);
                                    if inside && reveal && i == 20 {
                                        ui.scroll_to_rect(b.rect);
                                    }
                                    if i == 20 {
                                        ids.push(b.id);
                                    }
                                });
                            }
                        });
                        *target = Some(out.items[20]);
                        if !inside && reveal {
                            ui.scroll_to_rect(out.items[20]);
                        }
                    });
            });
            ids
        };
        pass(&mut c, false, &mut target);
        let ids = pass(&mut c, true, &mut target);
        pass(&mut c, false, &mut target);
        let ids = [ids, pass(&mut c, false, &mut target)].concat();
        let shown = c
            .probe()
            .previous_hits
            .iter()
            .find(|h| h.id == ids[0])
            .map(|h| h.rect.intersect(h.clip))
            .unwrap_or_default();
        assert!(
            !shown.is_empty() && shown.size().y >= 19.0,
            "item 20 is in view (inside: {inside}): {shown:?}"
        );
    }
}
