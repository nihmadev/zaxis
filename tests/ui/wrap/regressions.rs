//! Ordinary rows, columns, fill and size scopes share the flow that wrapping extends: they
//! keep their geometry and their input.
use super::*;

fn spacing(c: &Context) -> f32 {
    c.style().spacing
}

#[test]
fn a_row_advances_by_item_and_spacing_and_a_column_stacks() {
    let mut c = setup();
    let gap = spacing(&c);
    let (row, column) = root(&mut c, |ui| {
        let row = ui.horizontal(|ui| {
            [("r0", 30.0, 10.0), ("r1", 40.0, 20.0), ("r2", 20.0, 5.0)]
                .map(|(key, w, h)| block(ui, key, w, h).id)
        });
        let column = ui.vertical(|ui| {
            [("c0", 30.0, 10.0), ("c1", 40.0, 20.0)].map(|(key, w, h)| block(ui, key, w, h).id)
        });
        (row, column)
    });
    let rows = row.map(|id| placed(&c, id));
    assert_eq!(rows[0].min.x, 0.0);
    assert_eq!(rows[1].min.x, 30.0 + gap);
    assert_eq!(rows[2].min.x, 30.0 + gap + 40.0 + gap);
    assert!(rows.iter().all(|r| r.min.y == 0.0));
    let cols = column.map(|id| placed(&c, id));
    assert_eq!(cols[0].min.x, cols[1].min.x);
    assert_eq!(cols[1].min.y, cols[0].max.y + gap);
}

#[test]
fn aligned_rows_and_size_scopes_keep_their_rules() {
    let mut c = setup();
    let ids = root(&mut c, |ui| {
        let centered = ui.horizontal_aligned(Align::Center, |ui| {
            [
                block(ui, "tall", 20.0, 40.0).id,
                block(ui, "short", 20.0, 10.0).id,
            ]
        });
        let exact = ui.with_width(100.0, |ui| block(ui, "exact", 30.0, 10.0).id);
        let after = block(ui, "after", 10.0, 10.0).id;
        (centered, exact, after)
    });
    let (tall, short) = (placed(&c, ids.0[0]), placed(&c, ids.0[1]));
    assert_eq!(
        short.min.y - tall.min.y,
        15.0,
        "centered in the tallest item"
    );
    let (exact, after) = (placed(&c, ids.1), placed(&c, ids.2));
    assert_eq!(exact.min.x, 0.0);
    assert!(
        after.min.y >= exact.max.y,
        "the scope reserves its content, the next item follows"
    );
}

#[test]
fn fill_and_spacer_in_a_row_share_the_remaining_width_as_before() {
    let mut c = setup();
    let pass = |c: &mut Context| {
        root(c, |ui| {
            ui.with_width(300.0, |ui| {
                ui.horizontal(|ui| {
                    let left = block(ui, "left", 40.0, 10.0).id;
                    ui.spacer();
                    let right = block(ui, "right", 40.0, 10.0).id;
                    (left, right)
                })
            })
        })
    };
    pass(&mut c);
    let (left, right) = pass(&mut c);
    assert_eq!(placed(&c, left).min.x, 0.0);
    assert!((placed(&c, right).max.x - 300.0).abs() < 0.01);
    // Input reaches the right-hand block where it is shown.
    click_on(&mut c, right);
    let mut clicked = false;
    root(&mut c, |ui| {
        ui.with_width(300.0, |ui| {
            ui.horizontal(|ui| {
                block(ui, "left", 40.0, 10.0);
                ui.spacer();
                clicked = block(ui, "right", 40.0, 10.0).clicked();
            })
        })
    });
    assert!(clicked);
}

#[test]
fn a_wrap_inside_a_row_takes_the_remaining_width() {
    let mut c = setup();
    let out = root(&mut c, |ui| {
        ui.horizontal(|ui| {
            block(ui, "lead", 100.0, 10.0);
            Wrap::new("w").gap(4.0).show(ui, |ui| {
                for key in ["a", "b", "c", "d"] {
                    block(ui, key, 60.0, 8.0);
                }
            })
        })
    });
    let wrap = out;
    assert!(wrap.rect.min.x >= 100.0);
    assert!(wrap.rect.size().x <= 800.0 - 100.0 + 0.01);
}
