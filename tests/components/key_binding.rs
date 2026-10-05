use winit::keyboard::KeyCode;
use zaxis::components::key_box::binding::*;

#[test]
fn labels_follow_menu_conventions() {
    for (binding, label) in [
        (KeyBinding::None, "None"),
        (KeyBinding::Key(KeyCode::KeyE), "E"),
        (KeyBinding::Key(KeyCode::Digit3), "3"),
        (KeyBinding::Key(KeyCode::ShiftRight), "RShift"),
        (KeyBinding::Key(KeyCode::ControlLeft), "LCtrl"),
        (KeyBinding::Key(KeyCode::PageDown), "PgDn"),
        (KeyBinding::Key(KeyCode::Numpad5), "Num5"),
        (KeyBinding::Key(KeyCode::F11), "F11"),
        (KeyBinding::Key(KeyCode::Space), "Space"),
        (KeyBinding::Mouse(MouseBinding::Right), "RMB"),
    ] {
        assert_eq!(binding.label(), label);
    }
}

#[test]
fn held_and_pressed_read_the_input_state() {
    let mut input = zaxis::InputState::default();
    let key = KeyBinding::Key(KeyCode::KeyF);
    assert!(!key.is_down(&input) && !key.is_pressed(&input));
    input.keys_down.insert(KeyCode::KeyF);
    input.keys_pressed.insert(KeyCode::KeyF);
    assert!(key.is_down(&input) && key.is_pressed(&input));
    input.secondary_down = true;
    assert!(KeyBinding::Mouse(MouseBinding::Right).is_down(&input));
    assert!(!KeyBinding::None.is_down(&input));
}
