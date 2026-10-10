use super::*;

#[test]
fn dock_does_not_install_a_delete_shortcut_for_closing() {
    let mut s = Scene::new(true, 1.0);
    let tab = s.tab(p(0));
    s.click(tab.min + vec2(12.0, 12.0));
    s.events.clear();
    s.c.key(KeyCode::Delete, ElementState::Pressed, false);
    s.frame();
    s.c.key(KeyCode::Delete, ElementState::Released, false);
    s.frame();
    assert!(s.state.contains(p(0)));
    assert!(!s
        .events
        .iter()
        .any(|event| matches!(event, DockEvent::Closed { .. })));
}
#[test]
fn child_focus_activates_panel_and_geometric_shortcuts_move_once() {
    let mut s = Scene::new(true, 1.0);
    let button = s.controls[&p(2)];
    s.click(button.rect.center());
    assert_eq!(s.state.focused, Some(p(2)));
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Activated { panel } if *panel==p(2)))
            .count(),
        1
    );
    s.c.set_modifiers(ModifiersState::CONTROL | ModifiersState::ALT);
    s.c.key(KeyCode::ArrowLeft, ElementState::Pressed, false);
    s.c.key(KeyCode::ArrowLeft, ElementState::Released, false);
    s.frame();
    assert_eq!(s.state.focused, Some(p(0)));
    s.c.set_modifiers(ModifiersState::CONTROL | ModifiersState::SHIFT);
    s.c.key(KeyCode::ArrowRight, ElementState::Pressed, false);
    s.c.key(KeyCode::ArrowRight, ElementState::Released, false);
    s.frame();
    s.frame();
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Moved { panel,.. } if *panel==p(0)))
            .count(),
        1
    );
}
#[test]
fn close_button_emits_one_event_and_hidden_viewer_never_runs() {
    let mut s = Scene::new(true, 1.0);
    let tab = s.tab(p(1));
    s.click(vec2(tab.max.x - 16.0, tab.center().y));
    s.frame();
    s.frame();
    assert!(!s.state.contains(p(1)), "{:?}", s.events);
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Closed { panel } if *panel==p(1)))
            .count(),
        1
    );
    assert!(!s.built.contains(&p(1)));
    assert!(!s.built.contains(&p(3)));
}

#[test]
fn text_child_does_not_consume_dock_navigation_and_actions_close_once() {
    let mut s = Scene::new(true, 1.0);
    s.text = Some("alpha beta".into());
    s.frame();
    s.frame();
    let field = s.controls[&p(0)];
    s.click(field.rect.center());
    s.c.set_modifiers(ModifiersState::CONTROL | ModifiersState::ALT);
    assert!(s.c.key(KeyCode::ArrowRight, ElementState::Pressed, false));
    s.c.key(KeyCode::ArrowRight, ElementState::Released, false);
    s.frame();
    assert_eq!(s.state.focused, Some(p(2)));
    assert_eq!(s.text.as_deref(), Some("alpha beta"));
    let close = Action::new("dock.close", "Close panel").shortcut(Mods::PRIMARY.key(KeyCode::KeyW));
    s.actions.close = Some(close.id());
    s.c.set_actions(Actions::new().register(close));
    s.frame();
    s.c.set_modifiers(ModifiersState::CONTROL);
    s.c.key(KeyCode::KeyW, ElementState::Pressed, false);
    s.c.key(KeyCode::KeyW, ElementState::Released, false);
    s.frame();
    s.frame();
    assert!(!s.state.contains(p(2)));
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Closed { panel } if *panel==p(2)))
            .count(),
        1
    );
}

#[test]
fn tab_navigation_has_one_strip_stop_and_modifier_navigation_does_not_select_twice() {
    let mut s = Scene::new(true, 1.0);
    s.click(s.tab(p(2)).min + vec2(20.0, 16.0));
    s.events.clear();
    s.c.set_modifiers(ModifiersState::CONTROL | ModifiersState::ALT);
    s.c.key(KeyCode::ArrowLeft, ElementState::Pressed, false);
    s.c.key(KeyCode::ArrowLeft, ElementState::Released, false);
    s.frame();
    s.frame();
    assert_eq!(s.state.focused, Some(p(0)));
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e, DockEvent::Activated { .. }))
            .count(),
        1
    );
    let focusable: Vec<_> =
        s.c.probe()
            .previous_hits
            .iter()
            .filter(|h| h.action.focusable())
            .collect();
    assert_eq!(
        focusable.len(),
        5,
        "two tab strips, two child buttons and a split separator"
    );
}

#[test]
fn context_menu_float_and_split_are_real_operations() {
    let mut s = Scene::new(true, 1.0);
    let at = s.tab(p(1)).min + vec2(20.0, 16.0);
    s.c.move_pointer(at);
    s.c.secondary_button(ElementState::Pressed);
    s.c.secondary_button(ElementState::Released);
    s.frame();
    s.frame();
    assert!(s.c.probe().popup.is_some());
    // Shared ContextMenu keys choose its enabled Float row.
    for code in [KeyCode::ArrowDown, KeyCode::ArrowDown, KeyCode::Enter] {
        s.c.key(code, ElementState::Pressed, false);
        s.c.key(code, ElementState::Released, false);
        s.frame();
    }
    s.frame();
    assert!(
        s.state.floats.iter().any(|f| f.node.contains(p(1))),
        "{:?}",
        s.events
    );
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Floated { panel } if *panel==p(1)))
            .count(),
        1
    );
    let at = s.tab(p(3)).min + vec2(20.0, 16.0);
    s.c.move_pointer(at);
    s.c.secondary_button(ElementState::Pressed);
    s.c.secondary_button(ElementState::Released);
    s.frame();
    s.frame();
    for code in [
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::Enter,
    ] {
        s.c.key(code, ElementState::Pressed, false);
        s.c.key(code, ElementState::Released, false);
        s.frame();
    }
    s.frame();
    assert_eq!(
        s.events
            .iter()
            .filter(|e| matches!(e,DockEvent::Split { panel,.. } if *panel==p(3)))
            .count(),
        1
    );
}
