//! What changes inside items: text, and a focus group that is one item.
use super::*;
use zaxis::{FocusGroup, Text};

#[test]
fn a_text_change_reflows_the_lines_on_the_pass_that_sees_it() {
    let mut c = setup();
    let pass = |c: &mut Context, middle: &str| {
        root(c, |ui| {
            Wrap::new("w").width(200.0).gap(6.0).show(ui, |ui| {
                ui.add(Text::new("first"));
                ui.add(Text::new(middle.to_owned()));
                ui.add(Text::new("last"));
            })
        })
        .items
    };
    let short = pass(&mut c, "mid");
    assert_eq!(short.iter().map(|r| r.min.y).fold(0.0, f32::max), 0.0);
    let long = pass(&mut c, "a much longer label that cannot share the line");
    assert!(long[2].min.y > 0.0, "the last item moved down at once");
    assert!(long[1].size().x > short[1].size().x);
    assert_eq!(
        pass(&mut c, "a much longer label that cannot share the line"),
        long
    );
    assert_eq!(pass(&mut c, "mid"), short, "and back");
    assert!(c.next_repaint().is_none());
}

#[test]
fn a_focus_group_in_an_item_is_one_tab_stop_with_arrow_navigation() {
    let pass = |c: &mut Context, width: f32| {
        root(c, |ui| {
            Wrap::new("w").width(width).show(ui, |ui| {
                let before = block(ui, "before", 120.0, 20.0).id;
                let members = ui.item("tools", |ui| {
                    FocusGroup::new("tools")
                        .show(ui, |ui| {
                            ["a", "b", "c"].map(|key| block(ui, key, 40.0, 20.0).id)
                        })
                        .inner
                });
                let after = block(ui, "after", 60.0, 20.0).id;
                (before, members, after)
            })
        })
        .inner
    };
    for width in [500.0, 150.0] {
        // A fresh context: a group remembers the member focus was last on.
        let mut c = setup();
        let (before, members, after) = pass(&mut c, width);
        c.set_focus(Some(before));
        pass(&mut c, width);
        let tab = |c: &mut Context| {
            c.on_key_event(KeyCode::Tab, ElementState::Pressed, false);
            c.on_key_event(KeyCode::Tab, ElementState::Released, false);
            pass(c, width);
            c.probe().focused_widget
        };
        let entered = tab(&mut c);
        assert_eq!(
            entered,
            Some(members[0]),
            "Tab enters the group at its first member, width {width}"
        );
        c.on_key_event(KeyCode::ArrowRight, ElementState::Pressed, false);
        c.on_key_event(KeyCode::ArrowRight, ElementState::Released, false);
        pass(&mut c, width);
        assert_eq!(
            c.probe().focused_widget,
            Some(members[1]),
            "arrows move inside, width {width}"
        );
        assert_eq!(tab(&mut c), Some(after), "one Tab leaves the whole group");
    }
}
