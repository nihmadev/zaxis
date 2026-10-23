use super::*;
use zaxis::{DiagnosticKind, Text, TextEdit};

#[test]
fn ordinary_widgets_are_one_item_each_and_keep_their_own_size() {
    let mut c = setup();
    let labels = ["Alpha", "Be", "Gamma ray", "D"];
    // The size of each button alone in a row is the size it keeps in the wrap.
    let alone = root(&mut c, |ui| {
        ui.horizontal(|ui| labels.map(|l| ui.button(l).rect.size()))
    });
    let out = root(&mut c, |ui| {
        Wrap::new("w")
            .width(120.0)
            .show(ui, |ui| labels.map(|l| ui.button(l).rect.size()))
    });
    assert_eq!(out.inner, alone);
    assert_eq!(out.items.len(), 4);
    for (item, size) in out.items.iter().zip(alone) {
        assert_eq!(item.size(), size);
    }
    assert!(out.lines.len() > 1, "the labels do not fit in 120 px");
    let total: f32 = out.lines.iter().map(|l| l.size().y).sum();
    assert!(
        total > 0.0 && out.rect.size().y > total,
        "gaps lie between lines"
    );
}

#[test]
fn a_wrapped_button_is_clicked_where_it_is_shown_and_only_once() {
    let mut c = setup();
    let mut clicks = [0_u32; 6];
    let frame = |c: &mut Context, clicks: &mut [u32; 6]| {
        root(c, |ui| {
            Wrap::new("w").width(150.0).show(ui, |ui| {
                (0..6)
                    .map(|i| {
                        let r = ui.button(format!("Tag {i}"));
                        if r.clicked() {
                            clicks[i] += 1;
                        }
                        r.id
                    })
                    .collect::<Vec<_>>()
            })
        })
    };
    let out = frame(&mut c, &mut clicks);
    let target = placed(&c, out.inner[4]);
    assert!(target.min.y > 0.0, "the fifth button is on a later line");
    click_at(&mut c, target.center());
    frame(&mut c, &mut clicks);
    frame(&mut c, &mut clicks);
    assert_eq!(clicks, [0, 0, 0, 0, 1, 0]);
    // The place it was laid out at (the origin of the wrap) is not it.
    click_at(&mut c, vec2(2.0, 2.0));
    frame(&mut c, &mut clicks);
    assert_eq!(clicks, [1, 0, 0, 0, 1, 0], "only the first button is there");
}

#[test]
fn a_compound_item_is_never_split_and_moves_as_one() {
    let mut c = setup();
    let mut text = String::from("abc");
    let mut ids = None;
    let out = root(&mut c, |ui| {
        Wrap::new("w").width(260.0).show(ui, |ui| {
            block(ui, "lead", 100.0, 20.0);
            ids = Some(ui.item("filter", |ui| {
                let a = ui.add(TextEdit::new(&mut text).width(120.0)).id;
                let b = ui.button("Apply").id;
                (a, b)
            }));
        })
    });
    let (field, button) = ids.unwrap();
    let (field, button) = (placed(&c, field), placed(&c, button));
    assert!(
        field.min.y > 0.0,
        "100 + gap + (field + button) does not fit in 260"
    );
    assert_eq!(field.min.x, 0.0);
    assert!(
        button.min.x >= field.max.x,
        "the button follows the field in the item"
    );
    assert!((button.center().y - field.center().y).abs() < 20.0);
    assert_eq!(out.items.len(), 2, "two items, not three");
    assert!(out.items[1].contains(field.center()) && out.items[1].contains(button.center()));
}

#[test]
fn horizontal_vertical_and_nested_scopes_are_single_items() {
    let mut c = setup();
    let out = root(&mut c, |ui| {
        Wrap::new("w").width(100.0).gap(4.0).show(ui, |ui| {
            ui.horizontal(|ui| {
                block(ui, "h1", 30.0, 10.0);
                block(ui, "h2", 30.0, 10.0);
            });
            ui.vertical(|ui| {
                block(ui, "v1", 20.0, 10.0);
                block(ui, "v2", 20.0, 10.0);
            });
            ui.with_width(40.0, |ui| block(ui, "w", 10.0, 5.0));
        })
    });
    assert_eq!(out.items.len(), 3);
    assert_eq!(
        out.items[0].size(),
        vec2(30.0 + ui_spacing(&c) + 30.0, 10.0)
    );
    assert_eq!(out.items[1].size(), vec2(20.0, 10.0 * 2.0 + ui_spacing(&c)));
    assert_eq!(out.items[2].size(), vec2(40.0, 5.0));
}

fn ui_spacing(c: &Context) -> f32 {
    c.style().spacing
}

#[test]
fn an_item_wider_than_the_wrap_gets_its_own_line_and_is_not_shrunk() {
    let mut c = setup();
    let out = root(&mut c, |ui| {
        Wrap::new("w").width(100.0).gap(5.0).show(ui, |ui| {
            block(ui, "a", 40.0, 10.0);
            block(ui, "wide", 180.0, 12.0);
            block(ui, "b", 40.0, 10.0);
            block(ui, "c", 40.0, 10.0);
        })
    });
    assert_eq!(out.lines.len(), 3);
    near(out.items[1], rect(0.0, 15.0, 180.0, 12.0));
    near(out.items[2], rect(0.0, 32.0, 40.0, 10.0));
    near(out.items[3], rect(45.0, 32.0, 40.0, 10.0));
    assert_eq!(out.rect.size().x, 180.0);
}

#[test]
fn text_that_wraps_itself_gets_the_width_of_the_wrap_and_a_finite_height() {
    let mut c = setup();
    let long = "tags that keep going and going and going until they must break across lines";
    let out = root(&mut c, |ui| {
        Wrap::new("w").width(160.0).show(ui, |ui| {
            let wrapped = ui.add(Text::new(long).wrap(true)).rect.size();
            let narrow =
                ui.with_max_width(80.0, |ui| ui.add(Text::new(long).wrap(true)).rect.size());
            let one = ui.add(Text::new("short")).rect.size();
            (wrapped, narrow, one)
        })
    });
    let (wrapped, narrow, one) = out.inner;
    assert!(
        wrapped.x <= 160.0 + 0.01 && wrapped.y > one.y * 1.5,
        "{wrapped:?}"
    );
    assert!(
        narrow.x <= 80.0 + 0.01 && narrow.y > wrapped.y,
        "{narrow:?}"
    );
    assert!(out.rect.is_finite());
}

#[test]
fn fill_is_a_full_width_item_and_spacer_is_reported_and_ignored() {
    let mut c = setup();
    let out = root(&mut c, |ui| {
        Wrap::new("w").width(200.0).gap(5.0).show(ui, |ui| {
            block(ui, "a", 30.0, 10.0);
            ui.fill(|ui| {
                block(ui, "f", 10.0, 10.0);
            });
            ui.spacer();
            block(ui, "b", 30.0, 10.0);
        })
    });
    assert_eq!(out.items.len(), 3);
    near(out.items[1], rect(0.0, 15.0, 200.0, 10.0));
    near(out.items[2], rect(0.0, 30.0, 30.0, 10.0));
    let kinds: Vec<_> = c.diagnostics().iter().map(|d| d.kind).collect();
    assert_eq!(kinds, [DiagnosticKind::InvalidUsage]);
}

#[test]
fn fill_and_spacer_still_work_inside_an_item_of_a_wrap() {
    let mut c = setup();
    let pass = |c: &mut Context| {
        root(c, |ui| {
            Wrap::new("w").width(300.0).show(ui, |ui| {
                ui.with_width(200.0, |ui| {
                    ui.horizontal(|ui| {
                        let left = block(ui, "left", 20.0, 10.0).id;
                        ui.spacer();
                        [left, block(ui, "right", 20.0, 10.0).id]
                    })
                })
            })
        })
        .inner
    };
    // The spacer's share is resolved from the previous pass.
    pass(&mut c);
    let ids = pass(&mut c);
    assert_eq!(placed(&c, ids[0]).min.x, 0.0);
    assert!((placed(&c, ids[1]).max.x - 200.0).abs() < 0.01);
    assert!(c.diagnostics().is_empty());
}

#[test]
fn the_closure_and_every_item_run_once_per_pass() {
    let mut c = setup();
    let (mut outer, mut items, mut buttons) = (0, [0; 5], 0);
    for _ in 0..3 {
        root(&mut c, |ui| {
            Wrap::new("w").width(90.0).show(ui, |ui| {
                outer += 1;
                for (i, count) in items.iter_mut().enumerate() {
                    ui.item(i, |ui| {
                        *count += 1;
                        block(ui, "x", 40.0, 10.0);
                    });
                }
                buttons += ui.button("b").clicked() as u32;
            })
        });
    }
    assert_eq!((outer, items), (3, [3; 5]));
    assert_eq!(buttons, 0);
}

#[test]
fn nested_wraps_wrap_independently() {
    let mut c = setup();
    let out = root(&mut c, |ui| {
        Wrap::new("outer").width(200.0).show(ui, |ui| {
            block(ui, "lead", 190.0, 10.0);
            Wrap::new("inner").width(100.0).gap(5.0).show(ui, |ui| {
                for key in ["a", "b", "c"] {
                    block(ui, key, 45.0, 8.0);
                }
            })
        })
    });
    let inner = out.inner;
    assert_eq!(inner.lines.len(), 2, "a and b share a line, c wraps");
    assert_eq!(out.items.len(), 2);
    assert_eq!(out.items[1].size(), inner.rect.size());
    assert!(
        out.items[1].min.y > 10.0,
        "the inner wrap does not fit beside the lead"
    );
}

#[test]
fn ids_do_not_depend_on_the_line_an_item_lands_on() {
    let mut c = setup();
    let ids = |c: &mut Context, width: f32| {
        root(c, |ui| {
            Wrap::new("w")
                .width(width)
                .show(ui, |ui| ["x", "y", "z"].map(|t| ui.button(t).id))
        })
        .inner
    };
    assert_eq!(ids(&mut c, 500.0), ids(&mut c, 40.0));
}
