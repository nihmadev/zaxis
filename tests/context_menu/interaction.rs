use super::*;
use crate::{vec2, Button, ContextMenu, ContextMenuItem, ContextMenuOutput, Root, Text};
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(500, 350), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}
fn items() -> Vec<ContextMenuItem> {
    vec![
        ContextMenuItem::new("a", "Same")
            .icon("+")
            .left_text("01")
            .right_text("Ctrl+A"),
        ContextMenuItem::separator(),
        ContextMenuItem::new("disabled", "Disabled").enabled(false),
        ContextMenuItem::new("b", "Same").right_text("Ctrl+B"),
    ]
}
fn draw(
    c: &mut Context,
    items: &[ContextMenuItem],
    at: Option<Vec2>,
    passive: bool,
) -> (crate::Response, ContextMenuOutput) {
    let mut result = None;
    c.run(|c| {
        Root::new().show(c, |ui| {
            let target = if passive {
                ui.add(Text::new("Target"))
            } else {
                ui.add(Button::new("Target").min_size(vec2(300.0, 100.0)))
            };
            let mut menu = ContextMenu::new("test", items);
            if let Some(p) = at {
                menu = menu.open_at(p);
            }
            let output = menu.show(ui, target);
            result = Some((target, output));
        });
    });
    result.unwrap()
}
fn right(c: &mut Context, p: Vec2) -> bool {
    c.move_pointer(p);
    let consumed = c.secondary_button(ElementState::Pressed);
    c.secondary_button(ElementState::Released);
    consumed
}
fn click(c: &mut Context, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
}
fn key(c: &mut Context, code: KeyCode) {
    assert!(
        c.on_key_event(code, ElementState::Pressed, false).consumed,
        "key {code:?}, popup {:?}",
        c.popup.as_ref().map(|p| p.id)
    );
    c.on_key_event(code, ElementState::Released, false);
}
fn rows(c: &Context) -> Vec<HitRegion> {
    let popup = c.popup.as_ref().unwrap().id;
    c.previous_hits
        .iter()
        .filter(|h| {
            h.window == popup
                && matches!(h.action, HitAction::Activate | HitAction::Block)
                && h.rect.size().y < 50.0
        })
        .copied()
        .collect()
}

#[test]
fn right_press_opens_at_press_position_and_left_click_still_works() {
    let mut c = setup();
    let items = items();
    let (target, _) = draw(&mut c, &items, None, false);
    click(&mut c, target.rect.center());
    assert!(draw(&mut c, &items, None, false).0.clicked());
    let p = target.rect.min + vec2(22.0, 25.0);
    assert!(right(&mut c, p));
    c.move_pointer(vec2(400.0, 300.0));
    let output = draw(&mut c, &items, None, false).1;
    assert!(output.open);
    assert_eq!(output.rect.unwrap().min, p);
    assert!(output.selected.is_none());
}

#[test]
fn passive_target_and_distinct_duplicate_labels() {
    let mut c = setup();
    let items = items();
    let (target, _) = draw(&mut c, &items, None, true);
    assert!(right(&mut c, target.rect.center()));
    assert!(draw(&mut c, &items, None, true).1.open);
    let rows = rows(&c);
    assert_eq!(rows.len(), 3); // Separator has no interactive row.
    click(&mut c, rows[2].rect.center());
    let output = draw(&mut c, &items, None, true).1;
    assert_eq!(output.selected, Some(Id::new("b")));
    assert!(!output.open);
    assert!(c.popup.is_none());
    assert!(draw(&mut c, &items, None, true).1.selected.is_none());
}

#[test]
fn disabled_and_separator_do_not_activate_or_close() {
    let mut c = setup();
    let items = items();
    draw(&mut c, &items, Some(vec2(100.0, 100.0)), false);
    let r = rows(&c);
    click(&mut c, r[1].rect.center());
    let output = draw(&mut c, &items, None, false).1;
    assert!(output.open);
    assert!(output.selected.is_none());
    click(&mut c, vec2(r[0].rect.center().x, r[0].rect.max.y + 4.0));
    let output = draw(&mut c, &items, None, false).1;
    assert!(output.open);
    assert!(output.selected.is_none());
}

#[test]
fn keyboard_skips_disabled_and_preserves_event_order() {
    let mut c = setup();
    let items = items();
    draw(&mut c, &items, Some(vec2(100.0, 100.0)), false);
    key(&mut c, KeyCode::ArrowDown);
    key(&mut c, KeyCode::Enter);
    assert_eq!(
        draw(&mut c, &items, None, false).1.selected,
        Some(Id::new("b"))
    );
    draw(&mut c, &items, Some(vec2(100.0, 100.0)), false);
    key(&mut c, KeyCode::ArrowUp);
    key(&mut c, KeyCode::Enter);
    assert_eq!(
        draw(&mut c, &items, None, false).1.selected,
        Some(Id::new("b"))
    );
}

#[test]
fn escape_outside_click_and_focus_restore() {
    let mut c = setup();
    let items = items();
    let (target, _) = draw(&mut c, &items, None, false);
    c.request_focus(target.id);
    draw(&mut c, &items, Some(vec2(200.0, 150.0)), false);
    key(&mut c, KeyCode::Escape);
    assert!(!draw(&mut c, &items, None, false).1.open);
    assert_eq!(c.focused_widget, Some(target.id));
    draw(&mut c, &items, Some(vec2(200.0, 150.0)), false);
    click(&mut c, target.rect.min + vec2(3.0, 3.0));
    let (target, output) = draw(&mut c, &items, None, false);
    assert!(!output.open);
    assert!(!target.clicked());
}

#[test]
fn fits_viewport_and_scrolls_long_menus() {
    let mut c = setup();
    let items: Vec<_> = (0..50)
        .map(|i| ContextMenuItem::new(i, format!("Item {i}")))
        .collect();
    let output = draw(&mut c, &items, Some(vec2(495.0, 345.0)), false).1;
    let rect = output.rect.unwrap();
    assert!(rect.min.x >= 0.0 && rect.min.y >= 0.0 && rect.max.x <= 500.0 && rect.max.y <= 350.0);
    key(&mut c, KeyCode::End);
    draw(&mut c, &items, None, false);
    draw(&mut c, &items, None, false);
    key(&mut c, KeyCode::Enter);
    assert_eq!(
        draw(&mut c, &items, None, false).1.selected,
        Some(Id::new(49))
    );
}

#[test]
fn absent_target_cleans_retained_state_and_popup() {
    let mut c = setup();
    draw(&mut c, &items(), Some(vec2(100.0, 100.0)), false);
    c.run(|c| {
        Root::new().show(c, |_| {});
    });
    assert!(c.context_menus.is_empty());
    assert!(c.popup.is_none());
}

#[test]
fn short_menu_has_no_overflow_and_long_captions_fit_their_columns() {
    let mut c = setup();
    let items = vec![
        ContextMenuItem::new("copy", "Duplicate")
            .icon("+")
            .right_text("Ctrl+D"),
        ContextMenuItem::separator(),
        ContextMenuItem::new("color", "Blue")
            .left_text("01")
            .right_text("On"),
        ContextMenuItem::new("delete", "Remove copy")
            .icon("-")
            .right_text("Del"),
    ];
    draw(&mut c, &items, Some(vec2(100.0, 100.0)), false);
    let popup = c.popup.as_ref().unwrap().id;
    assert!(!c
        .previous_hits
        .iter()
        .any(|h| h.window == popup && matches!(h.action, HitAction::ScrollThumb { .. })));
    for scroll in c.scrolling.states.values().filter(|s| s.window == popup) {
        assert_eq!(scroll.max_offset(), Vec2::ZERO);
    }
    for (id, element) in &c.cache {
        for paint in &element.paint {
            if let Paint::Text {
                text,
                size,
                position,
                ..
            } = paint
            {
                if text == "Remove copy" || text == "Ctrl+D" {
                    let width = c.text.measure(text, *size, f32::INFINITY).x;
                    let clip = c
                        .elements
                        .iter()
                        .find(|e| e.id == *id && e.layer == popup)
                        .unwrap()
                        .clip;
                    assert!(position.x >= clip.min.x - 0.01);
                    assert!(
                        position.x + width <= clip.max.x + 0.01,
                        "clipped caption {text}"
                    );
                }
            }
        }
    }
}

#[test]
fn hover_is_immediate_and_does_not_schedule_animation_frames() {
    let mut c = setup();
    let mut style = c.style().clone();
    style.motion.reduced_motion = false;
    c.set_style(style);
    let items = items();
    draw(&mut c, &items, Some(vec2(180.0, 150.0)), false);
    let rows = rows(&c);
    for row in [rows[0], rows[2], rows[0]] {
        c.move_pointer(row.rect.center());
        draw(&mut c, &items, None, false);
        assert!(!c.wants_animation_frame());
        let body = &c.cache[&row.id.with("body")].paint;
        assert!(body.iter().any(|p| matches!(p, Paint::Shape(crate::Shape::Rect { fill, .. }) if *fill == c.style().button_hovered)));
    }
}

#[test]
fn ten_thousand_mixed_rows_virtualize_and_refresh_after_mutation() {
    let mut c = setup();
    let mut items: Vec<_> = (0..10000)
        .map(|i| {
            if i % 7 == 6 {
                ContextMenuItem::separator()
            } else {
                ContextMenuItem::new(i, format!("Action {i}"))
            }
        })
        .collect();
    draw(&mut c, &items, Some(vec2(180.0, 150.0)), false);
    assert!(rows(&c).len() < 20);
    assert!(c.cache.len() < 150);
    key(&mut c, KeyCode::End);
    draw(&mut c, &items, None, false);
    let last = rows(&c)
        .into_iter()
        .filter(|r| r.action == HitAction::Activate)
        .last()
        .unwrap();
    assert!(!last.rect.intersect(last.clip).is_empty());
    key(&mut c, KeyCode::Enter);
    assert_eq!(
        draw(&mut c, &items, None, false).1.selected,
        Some(Id::new(9999))
    );
    items[0].text = "A changed and significantly longer action caption".into();
    items[0].enabled = false;
    let output = draw(&mut c, &items, Some(vec2(100.0, 100.0)), false).1;
    assert!(output.rect.unwrap().size().x > 220.0);
    key(&mut c, KeyCode::Home);
    key(&mut c, KeyCode::Enter);
    assert_eq!(
        draw(&mut c, &items, None, false).1.selected,
        Some(Id::new(1))
    );
}
