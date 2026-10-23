use super::*;

#[test]
fn items_flow_left_to_right_and_continue_on_the_next_line() {
    let mut c = setup();
    let (out, rects) = wrap_blocks(
        &mut c,
        250.0,
        vec2(10.0, 6.0),
        &[(100.0, 20.0), (100.0, 30.0), (100.0, 20.0), (40.0, 10.0)],
    );
    near(rects[0], rect(0.0, 0.0, 100.0, 20.0));
    near(rects[1], rect(110.0, 0.0, 100.0, 30.0));
    near(rects[2], rect(0.0, 36.0, 100.0, 20.0));
    near(rects[3], rect(110.0, 36.0, 40.0, 10.0));
    near(out.rect, rect(0.0, 0.0, 250.0, 56.0));
    assert_eq!(out.lines.len(), 2);
    near(out.lines[0], rect(0.0, 0.0, 210.0, 30.0));
    near(out.lines[1], rect(0.0, 36.0, 150.0, 20.0));
    assert_eq!(out.items.len(), 4);
    near(out.items[3], rects[3]);
}

#[test]
fn an_item_that_lands_exactly_on_the_edge_stays_on_the_line() {
    let mut c = setup();
    let (out, rects) = wrap_blocks(&mut c, 210.0, vec2(10.0, 4.0), &[(100.0, 20.0); 2]);
    near(rects[1], rect(110.0, 0.0, 100.0, 20.0));
    assert_eq!(out.lines.len(), 1, "no empty line is created");
    assert_eq!(out.rect.size().y, 20.0, "no gap after the last line");
}

#[test]
fn an_edge_hit_survives_fractional_rounding_but_a_real_overflow_wraps() {
    let mut c = setup();
    // Three thirds plus two gaps equal the width up to float rounding.
    let third = 100.0 / 3.0;
    let (out, _) = wrap_blocks(
        &mut c,
        100.0 + 2.0 * 7.0,
        vec2(7.0, 0.0),
        &[(third * 3.0 / 3.0, 5.0); 3],
    );
    assert_eq!(out.lines.len(), 1);
    let (out, _) = wrap_blocks(
        &mut c,
        100.0 + 2.0 * 7.0 - 0.01,
        vec2(7.0, 0.0),
        &[(third, 5.0); 3],
    );
    assert_eq!(out.lines.len(), 2, "0.01 over the edge is a real overflow");
}

#[test]
fn an_empty_wrap_occupies_nothing() {
    let mut c = setup();
    let out = root(&mut c, |ui| Wrap::new("w").width(200.0).show(ui, |_| ()));
    assert_eq!(out.rect.size(), vec2(200.0, 0.0));
    assert!(out.lines.is_empty() && out.items.is_empty());
}

#[test]
fn one_item_is_one_line() {
    let mut c = setup();
    let (out, rects) = wrap_blocks(&mut c, 200.0, vec2(8.0, 8.0), &[(50.0, 17.0)]);
    near(rects[0], rect(0.0, 0.0, 50.0, 17.0));
    assert_eq!(out.rect.size().y, 17.0);
}

#[test]
fn the_wrap_starts_at_the_cursor_of_its_parent_and_the_next_item_follows_its_height() {
    let mut c = setup();
    let (wrap, next, spacing) = root(&mut c, |ui| {
        let before = ui.allocate_space(vec2(10.0, 30.0));
        let out = Wrap::new("w").width(100.0).gap(5.0).show(ui, |ui| {
            for key in ["a", "b", "c"] {
                block(ui, key, 60.0, 10.0);
            }
        });
        assert_eq!(out.rect.min.y, before.max.y + ui.style().spacing);
        (
            out.rect,
            ui.allocate_space(vec2(1.0, 1.0)),
            ui.style().spacing,
        )
    });
    // Three 60-wide items in 100 px: one per line, a gap of 5 between lines.
    assert_eq!(wrap.size(), vec2(100.0, 40.0));
    assert_eq!(next.min.y, wrap.max.y + spacing);
}

#[test]
fn gaps_widths_and_sizes_are_sanitized_without_panicking() {
    let mut c = setup();
    let weird = [f32::NAN, f32::INFINITY, -5.0, 0.0, 1.0e30];
    for w in weird {
        for g in weird {
            let out = root(&mut c, |ui| {
                Wrap::new("w").width(w).spacing(vec2(g, g)).show(ui, |ui| {
                    block(ui, "a", 30.0, 10.0);
                    block(ui, "b", 30.0, 10.0);
                })
            });
            assert!(out.rect.is_finite(), "width {w} gap {g}");
            assert!(out.items.iter().chain(&out.lines).all(|r| r.is_finite()));
        }
    }
    let out = root(&mut c, |ui| {
        Wrap::new("w").width(100.0).show(ui, |ui| {
            block(ui, "a", f32::NAN, 10.0);
            block(ui, "b", f32::INFINITY, 4.0);
            block(ui, "c", 20.0, f32::NAN);
            block(ui, "d", 1.0e30, 1.0e30);
        })
    });
    assert!(out.rect.is_finite() && out.items.iter().all(|r| r.is_finite()));
}

#[test]
fn zero_width_gives_every_item_its_own_line() {
    let mut c = setup();
    let (out, rects) = wrap_blocks(&mut c, 0.0, vec2(4.0, 3.0), &[(10.0, 5.0); 3]);
    assert_eq!(out.lines.len(), 3);
    near(rects[2], rect(0.0, 16.0, 10.0, 5.0));
}
