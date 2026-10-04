//! Keyboard pick-up, navigation, confirm and cancel; text input is never taken.
use super::*;

fn focus_source(s: &mut Scene) {
    s.key(KeyCode::Tab);
    s.frame();
    assert!(s.context.focused_widget.is_some());
}

#[test]
fn space_picks_up_arrows_choose_a_target_and_enter_drops_it() {
    let mut s = scene(1.0);
    focus_source(&mut s);
    let focus = s.context.focused_widget;
    // Arrows belong to ordinary navigation until a drag is started explicitly.
    s.key(KeyCode::ArrowDown);
    assert!(s.context.drag.session.is_none());
    s.key(KeyCode::Space);
    assert!(s.context.drag.session.as_ref().is_some_and(|d| d.keyboard));
    s.frame();
    s.frame();
    assert_eq!((s.log.begins, s.seen.active), (1, true));
    assert!(!s.seen.hover);
    s.key(KeyCode::ArrowDown);
    s.frame();
    assert!(s.seen.hover && s.seen.acceptable);
    assert_eq!(s.context.drag_cursor(), winit::window::CursorIcon::Move);
    s.key(KeyCode::Enter);
    s.frame();
    assert_eq!(s.log.drops.len(), 1);
    assert_eq!(s.log.drops[0].payload, Item(7));
    s.frame();
    assert_eq!(s.log.ends[0].reason, DragReason::Dropped);
    assert_eq!(s.context.focused_widget, focus, "focus is never moved");
    assert!(s.context.drag.session.is_none());
}

#[test]
fn zones_give_keyboard_users_before_inside_and_after_stops() {
    let mut s = scene(1.0);
    s.zones = Some(DropZones::tree());
    s.frame();
    focus_source(&mut s);
    s.key(KeyCode::Space);
    s.frame();
    s.frame();
    let mut order = Vec::new();
    for _ in 0..4 {
        s.key(KeyCode::ArrowDown);
        s.frame();
        if let Some(zone) = s.seen.insertion {
            order.push(zone);
        }
    }
    assert_eq!(
        order,
        [
            Insertion::Before,
            Insertion::Inside,
            Insertion::After,
            Insertion::After
        ]
    );
    s.key(KeyCode::ArrowUp);
    s.frame();
    assert_eq!(s.seen.insertion, Some(Insertion::Inside));
    s.key(KeyCode::Space);
    s.frame();
    assert_eq!(s.log.drops[0].insertion, Some(Insertion::Inside));
}

#[test]
fn escape_and_tab_cancel_a_keyboard_drag() {
    for code in [KeyCode::Escape, KeyCode::Tab] {
        let mut s = scene(1.0);
        focus_source(&mut s);
        s.key(KeyCode::Space);
        s.frame();
        s.frame();
        s.key(KeyCode::ArrowDown);
        s.frame();
        s.key(code);
        s.frame();
        s.frame();
        assert!(s.log.drops.is_empty() && s.context.drag.session.is_none());
        assert_eq!(s.log.ends.len(), 1);
        assert_eq!(
            s.log.ends[0].reason,
            if code == KeyCode::Escape {
                DragReason::Escape
            } else {
                DragReason::Cancelled
            }
        );
    }
}

#[test]
fn text_edit_keeps_space_and_arrows_and_a_pointer_press_cancels_a_keyboard_drag() {
    let mut s = scene(1.0);
    s.with_text = true;
    s.frame();
    // Click the text field, then type: no drag is ever started from it.
    let edit = s
        .context
        .previous_hits
        .iter()
        .find(|h| h.action == HitAction::TextEdit)
        .unwrap()
        .rect;
    s.move_to(edit.center());
    s.press();
    s.release();
    s.frame();
    s.key(KeyCode::Space);
    s.context.on_text_event(" ");
    s.key(KeyCode::ArrowLeft);
    s.frame();
    assert!(s.context.drag.session.is_none());
    assert_eq!(s.text, " ");

    focus_source(&mut s);
    s.key(KeyCode::Space);
    s.frame();
    s.frame();
    assert!(s.context.drag.session.is_some());
    s.move_to(Vec2::new(700.0, 500.0));
    s.press();
    s.release();
    s.frame();
    s.frame();
    assert!(s.context.drag.session.is_none());
    assert_eq!(s.log.ends.len(), 1);
}
