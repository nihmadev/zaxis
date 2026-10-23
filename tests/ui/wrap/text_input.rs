use super::*;
use std::time::Duration;
use zaxis::{Button, ImeEvent, InputEvent, Popup, TextEdit, Tooltip};

struct Model {
    text: String,
    other: String,
    open: bool,
}

fn pass(c: &mut Context, m: &mut Model, width: f32) -> (WrapOutput<[Response; 3]>, Rect) {
    let mut trigger = Rect::default();
    let out = root(c, |ui| {
        Wrap::new("w").width(width).gap(6.0).show(ui, |ui| {
            let lead = ui.item("lead", |ui| block(ui, "lead", 90.0, 24.0));
            let field = ui.item("field", |ui| {
                ui.add(TextEdit::new(&mut m.text).id_source("field").width(130.0))
            });
            let menu = ui.item("menu", |ui| {
                let button = ui.add(Button::new("Menu"));
                trigger = button.rect;
                Popup::new("menu-popup", button.rect)
                    .size(vec2(120.0, 60.0))
                    .show(ui, &mut m.open, |ui| ui.label("Action"));
                if button.clicked() {
                    m.open = !m.open;
                }
                button
            });
            ui.item("other", |ui| {
                ui.add(TextEdit::new(&mut m.other).id_source("other").width(80.0));
            });
            [lead, field, menu]
        })
    });
    (out, trigger)
}

fn model() -> Model {
    Model {
        text: "hello".into(),
        other: String::new(),
        open: false,
    }
}

#[test]
fn a_popup_in_an_item_is_anchored_to_the_wrapped_trigger_and_not_clipped_by_the_line() {
    for width in [420.0, 250.0, 130.0] {
        let mut c = setup();
        let mut m = model();
        let (out, _) = pass(&mut c, &mut m, width);
        click_on(&mut c, out.inner[2].id);
        pass(&mut c, &mut m, width);
        let (out, _) = pass(&mut c, &mut m, width);
        assert!(m.open, "the wrapped trigger took the click at {width}");
        let trigger = placed(&c, out.inner[2].id);
        let popup = c.probe().popup.as_ref().expect("an open popup");
        near(popup.anchor, trigger);
        assert!(popup.rect.min.y >= trigger.max.y, "opens below at {width}");
        let wrap_bottom = out.rect.max.y;
        assert!(
            popup.rect.max.y > wrap_bottom || trigger.max.y + popup.rect.size().y > wrap_bottom,
            "the panel is not cut to its line"
        );
    }
}

#[test]
fn the_popup_follows_its_trigger_when_the_width_changes_and_lines_reflow() {
    let mut c = setup();
    let mut m = model();
    let (out, _) = pass(&mut c, &mut m, 420.0);
    click_on(&mut c, out.inner[2].id);
    pass(&mut c, &mut m, 420.0);
    pass(&mut c, &mut m, 420.0);
    let before = placed(&c, out.inner[2].id);
    let (out, _) = pass(&mut c, &mut m, 140.0);
    pass(&mut c, &mut m, 140.0);
    let after = placed(&c, out.inner[2].id);
    assert_ne!(before.min, after.min, "the trigger moved to another line");
    assert!(m.open, "a reflow does not close it");
    near(c.probe().popup.as_ref().unwrap().anchor, after);
}

#[test]
fn text_edit_in_a_later_line_takes_clicks_and_typing_at_its_displayed_text() {
    let mut c = setup();
    let mut m = model();
    let (out, _) = pass(&mut c, &mut m, 200.0);
    let field = placed(&c, out.inner[1].id);
    assert!(field.min.y > 0.0, "the field is on the second line");
    click_at(&mut c, field.center());
    pass(&mut c, &mut m, 200.0);
    c.on_text_event("!");
    pass(&mut c, &mut m, 200.0);
    assert!(m.text.contains('!'), "{}", m.text);
    assert_eq!(m.text.matches('!').count(), 1, "one input, one change");
    let state = &c.probe().text_edits[&out.inner[1].id];
    assert!(state.buffer.cursor > 0);
    // IME composition is reported at the displayed field, not at the layout origin.
    c.on_input(InputEvent::Ime(ImeEvent::Preedit("Ж".into(), Some((0, 2)))));
    pass(&mut c, &mut m, 200.0);
    let ime = c.probe().ime_area.expect("IME area while composing");
    assert!(field.contains(ime.center()), "{ime:?} outside {field:?}");
}

#[test]
fn reflow_keeps_focus_text_and_selection() {
    let mut c = setup();
    let mut m = model();
    let (out, _) = pass(&mut c, &mut m, 600.0);
    let field = out.inner[1].id;
    click_on(&mut c, field);
    pass(&mut c, &mut m, 600.0);
    c.on_text_event("X");
    pass(&mut c, &mut m, 600.0);
    let cursor = c.probe().text_edits[&field].buffer.cursor;
    let focused = c.probe().focused_widget;
    assert!(focused.is_some());
    for width in [300.0, 140.0, 90.0, 600.0] {
        let (out, _) = pass(&mut c, &mut m, width);
        assert!(out.inner[1].has_focus, "focus survives width {width}");
        assert_eq!(c.probe().focused_widget, focused);
        assert_eq!(c.probe().text_edits[&field].buffer.cursor, cursor);
        assert!(m.text.contains('X'));
    }
}

#[test]
fn tab_visits_items_in_declaration_order_whatever_the_lines() {
    let mut c = setup();
    let mut m = model();
    let (out, _) = pass(&mut c, &mut m, 200.0);
    let order = [out.inner[0].id, out.inner[1].id, out.inner[2].id];
    let mut visited = Vec::new();
    for _ in 0..3 {
        c.on_key_event(KeyCode::Tab, ElementState::Pressed, false);
        c.on_key_event(KeyCode::Tab, ElementState::Released, false);
        pass(&mut c, &mut m, 200.0);
        visited.push(c.probe().focused_widget);
    }
    // Focusable items in order: the custom block, the field, then the button.
    assert_eq!(visited[0], Some(order[0]));
    assert_eq!(visited[1], Some(order[1]));
    assert_eq!(visited[2], Some(order[2]));
}

#[test]
fn a_tooltip_on_a_wrapped_widget_appears_at_its_displayed_position() {
    let mut c = setup();
    let t0 = c.frame_time();
    let draw = |c: &mut Context, at: Instant| {
        let mut id = Id::new("none");
        c.run_at(at, |c| {
            root_inner(c, |ui| {
                Wrap::new("w").width(150.0).gap(6.0).show(ui, |ui| {
                    block(ui, "lead", 100.0, 20.0);
                    id = ui
                        .add(Tooltip::new("Explains the tag").wrap(Button::new("Tag")))
                        .id;
                });
            });
        });
        id
    };
    let id = draw(&mut c, t0);
    let target = placed(&c, id);
    assert!(target.min.y > 0.0, "the tagged button wrapped");
    c.move_pointer(target.center());
    draw(&mut c, t0);
    draw(&mut c, t0 + Duration::from_millis(500));
    let paint = &c.probe().cache[&id.with("tooltip")].paint;
    let Paint::Shape(Shape::Rect { rect: tip, .. }) = &paint[0] else {
        panic!("tooltip box");
    };
    // Next to the button where it is shown (below it, or above at an edge), not at the
    // origin of the wrap where the button was built.
    let beside = (tip.min.y - target.max.y).abs() < 40.0 || (target.min.y - tip.max.y).abs() < 40.0;
    let over = tip.min.x < target.max.x && tip.max.x > target.min.x;
    assert!(beside && over, "{tip:?} is not next to {target:?}");
}

fn root_inner(c: &mut Context, build: impl FnOnce(&mut Ui<'_>)) {
    Root::new().padding(Padding::all(0.0)).show(c, build);
}
