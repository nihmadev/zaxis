use super::*;
use zaxis::{InputEvent, TextEdit};

#[test]
fn reordering_keyed_items_keeps_each_owner_state_and_focus() {
    let mut c = setup();
    let mut texts = vec![
        ("a", String::from("alpha")),
        ("b", String::from("beta")),
        ("c", String::from("gamma")),
    ];
    let pass = |c: &mut Context, texts: &mut Vec<(&'static str, String)>| {
        root(c, |ui| {
            Wrap::new("w").width(180.0).show(ui, |ui| {
                texts
                    .iter_mut()
                    .map(|(key, text)| {
                        ui.item(*key, |ui| {
                            ui.add(TextEdit::new(text).id_source("edit").width(100.0))
                                .id
                        })
                    })
                    .collect::<Vec<_>>()
            })
        })
    };
    let out = pass(&mut c, &mut texts);
    let beta = out.inner[1];
    click_on(&mut c, beta);
    pass(&mut c, &mut texts);
    c.on_text_event("!");
    pass(&mut c, &mut texts);
    assert!(texts[1].1.contains('!'), "typing reached the focused field");
    // Reverse: "b" moves from the second to the middle of a different line layout.
    texts.reverse();
    let out = pass(&mut c, &mut texts);
    assert_eq!(
        out.inner[1], beta,
        "the same item has the same id in its new place"
    );
    assert!(c.probe().focused_widget == Some(beta));
    c.on_text_event("?");
    pass(&mut c, &mut texts);
    assert!(texts[1].1.contains('?') && texts[1].0 == "b");
    assert!(!texts[0].1.contains('?') && !texts[2].1.contains('?'));
}

#[test]
fn removing_the_focused_or_dragged_item_releases_focus_and_capture() {
    let mut c = setup();
    let mut present = true;
    let mut dragged = 0;
    let pass = |c: &mut Context, present: bool, dragged: &mut u32| {
        root(c, |ui| {
            Wrap::new("w").width(150.0).show(ui, |ui| {
                block(ui, "first", 60.0, 20.0);
                if present {
                    let rect = ui.allocate_space(vec2(60.0, 20.0));
                    let r = ui.interact(rect, "drag", Sense::DRAG | Sense::FOCUS);
                    *dragged += r.drag_started() as u32;
                    r.id
                } else {
                    Id::new("gone")
                };
                block(ui, "last", 60.0, 20.0);
            })
        })
    };
    pass(&mut c, present, &mut dragged);
    let target = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.action != HitAction::Block && h.rect.min.x > 60.0 && h.rect.min.y < 1.0)
        .copied()
        .unwrap();
    c.move_pointer(target.rect.center());
    c.primary_button(ElementState::Pressed);
    c.move_pointer(target.rect.center() + vec2(30.0, 30.0));
    pass(&mut c, present, &mut dragged);
    assert!(
        c.probe().capture.is_some(),
        "the press captured the pointer"
    );
    present = false;
    pass(&mut c, present, &mut dragged);
    pass(&mut c, present, &mut dragged);
    assert!(
        c.probe().capture.is_none(),
        "capture is released with its owner"
    );
    assert!(
        c.probe().focused_widget.is_none(),
        "focus is released with its owner"
    );
    c.primary_button(ElementState::Released);
    pass(&mut c, present, &mut dragged);
    assert_eq!(dragged, 1);
}

#[test]
fn state_follows_the_items_that_exist_and_idle_requests_nothing() {
    let mut c = setup();
    let pass = |c: &mut Context, count: usize| {
        root(c, |ui| {
            Wrap::new("w").width(300.0).show(ui, |ui| {
                for i in 0..count {
                    ui.push_id(i, |ui| ui.button(format!("Tag {i}")));
                }
            })
        })
    };
    for round in 0..6 {
        pass(&mut c, 400);
        pass(&mut c, 3);
        let live = c.probe().previous_hits.len();
        assert!(
            live < 40,
            "round {round}: {live} hit regions remain after unmounting"
        );
        assert!(c.probe().input_transforms.len() < 40);
        assert!(c.probe().cache.len() < 80, "{}", c.probe().cache.len());
    }
    for _ in 0..3 {
        pass(&mut c, 200);
    }
    let before = c.cache_stats();
    for _ in 0..3 {
        pass(&mut c, 200);
    }
    let after = c.cache_stats();
    assert_eq!(
        after.tessellated_elements, before.tessellated_elements,
        "idle reuses meshes"
    );
    assert_eq!(
        after.geometry_rebuilds, before.geometry_rebuilds,
        "and frame geometry"
    );
    assert!(
        c.next_repaint().is_none(),
        "nothing is scheduled while idle"
    );
}

#[test]
fn resize_and_style_changes_settle_in_one_pass() {
    let mut c = setup();
    let pass = |c: &mut Context| {
        root(c, |ui| {
            Wrap::new("w")
                .show(ui, |ui| {
                    for i in 0..14 {
                        ui.push_id(i, |ui| ui.button(format!("Filter {i}")));
                    }
                })
                .items
        })
    };
    let wide = pass(&mut c);
    let lines = |items: &[Rect]| {
        let mut ys: Vec<_> = items.iter().map(|r| r.min.y as i32).collect();
        ys.dedup();
        ys.len()
    };
    c.on_input(InputEvent::Resized {
        width: 320,
        height: 600,
    });
    let narrow = pass(&mut c);
    assert!(lines(&narrow) > lines(&wide));
    assert_eq!(pass(&mut c), narrow, "converged after one pass");
    assert!(narrow.iter().all(|r| r.max.x <= 320.0 + 0.01));
    c.on_input(InputEvent::Resized {
        width: 800,
        height: 600,
    });
    assert_eq!(pass(&mut c), wide);

    let mut style = c.style().clone();
    style.font_size *= 1.5;
    c.set_style(style);
    let large = pass(&mut c);
    assert!(lines(&large) >= lines(&wide));
    assert_ne!(large, wide);
    assert_eq!(pass(&mut c), large, "a style change also settles at once");
    assert!(c.next_repaint().is_none());
}
