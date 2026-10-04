use super::*;
use winit::keyboard::ModifiersState;

fn modal_focusables(c: &Context) -> Vec<Id> {
    let top = c.top_modal_id().unwrap();
    c.previous_hits
        .iter()
        .filter(|h| h.window == top && h.action.focusable())
        .map(|h| h.id)
        .collect()
}

fn focus_under(c: &mut Context, s: &mut Scene) -> Id {
    draw(c, s);
    let p = center(s.under);
    click(c, s, p);
    c.focused_widget.expect("the Under button takes focus")
}

#[test]
fn opening_focuses_the_first_control_and_closing_restores_the_trigger() {
    let mut c = setup();
    let mut s = Scene::default();
    let trigger = focus_under(&mut c, &mut s);
    open(&mut c, &mut s);
    let inside = modal_focusables(&c);
    assert!(inside.len() >= 3, "button, text field, combo box and close");
    assert_eq!(c.focused_widget, Some(inside[0]));
    assert!(!inside.contains(&trigger));
    press(&mut c, &mut s, KeyCode::Escape);
    assert!(!s.open);
    assert_eq!(c.focused_widget, Some(trigger));
}

#[test]
fn return_focus_to_a_vanished_widget_clears_focus() {
    let mut c = setup();
    let mut s = Scene::default();
    focus_under(&mut c, &mut s);
    open(&mut c, &mut s);
    s.show_under = false;
    draw(&mut c, &mut s);
    press(&mut c, &mut s, KeyCode::Escape);
    assert!(!s.open);
    assert_eq!(c.focused_widget, None);
}

#[test]
fn tab_and_shift_tab_cycle_only_inside_the_modal() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    open(&mut c, &mut s);
    let inside = modal_focusables(&c);
    let mut seen = vec![c.focused_widget.unwrap()];
    for _ in 0..inside.len() {
        assert!(press(&mut c, &mut s, KeyCode::Tab));
        seen.push(c.focused_widget.unwrap());
    }
    assert!(seen.iter().all(|id| inside.contains(id)), "{seen:?}");
    assert_eq!(seen.first(), seen.last(), "forward cycle wraps around");
    let mut unique = seen[..inside.len()].to_vec();
    unique.sort_by_key(|id| format!("{id:?}"));
    unique.dedup();
    assert_eq!(unique.len(), inside.len(), "every control is visited once");
    c.input.modifiers = ModifiersState::SHIFT;
    press(&mut c, &mut s, KeyCode::Tab);
    assert_eq!(c.focused_widget, Some(seen[inside.len() - 1]));
}

#[test]
fn focus_cannot_be_moved_under_the_modal() {
    let mut c = setup();
    let mut s = Scene::default();
    draw(&mut c, &mut s);
    open(&mut c, &mut s);
    let under = c
        .previous_hits
        .iter()
        .find(|h| h.action.focusable() && Some(h.window) != c.top_modal_id())
        .map(|h| h.id)
        .expect("a focusable control under the modal");
    c.request_focus(under);
    draw(&mut c, &mut s);
    let inside = modal_focusables(&c);
    assert!(inside.contains(&c.focused_widget.unwrap()));
}

#[test]
fn clicking_the_overlay_keeps_focus_inside() {
    let mut c = setup();
    let mut s = Scene {
        overlay: false,
        ..Scene::default()
    };
    open(&mut c, &mut s);
    let focused = c.focused_widget;
    assert!(focused.is_some());
    let p = center(s.under);
    click(&mut c, &mut s, p);
    assert_eq!(c.focused_widget, focused);
}

#[test]
fn enter_runs_the_default_action_only_outside_fields_and_buttons() {
    let mut c = setup();
    let mut s = Scene::default();
    open(&mut c, &mut s);
    // In a text field Enter belongs to the field.
    let p = center(s.inner_text_rect);
    click(&mut c, &mut s, p);
    press(&mut c, &mut s, KeyCode::Enter);
    assert_eq!(s.default_actions, 0);
    // On a button Enter activates that button, once.
    let p = center(s.inner);
    click(&mut c, &mut s, p);
    s.inner_clicks = 0;
    press(&mut c, &mut s, KeyCode::Enter);
    assert_eq!((s.inner_clicks, s.default_actions), (1, 0));
    // With nothing focused it is the modal's default action, once.
    c.set_focus(None);
    press(&mut c, &mut s, KeyCode::Enter);
    assert_eq!(s.default_actions, 1);
    assert!(
        s.open,
        "the default action is reported, not a close by itself"
    );
}
