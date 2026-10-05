use crate::prelude::*;
use winit::keyboard::{Key, NamedKey, PhysicalKey};
use zaxis::context::events::navigation_code;
#[test]
fn keypad_navigation_uses_logical_key_without_changing_numeric_input() {
    let physical = PhysicalKey::Code(KeyCode::Numpad1);
    assert_eq!(
        navigation_code(physical, &Key::Named(NamedKey::End)),
        Some(KeyCode::End)
    );
    assert_eq!(
        navigation_code(physical, &Key::Character("1".into())),
        Some(KeyCode::Numpad1)
    );
    assert_eq!(
        navigation_code(
            PhysicalKey::Code(KeyCode::Numpad2),
            &Key::Named(NamedKey::ArrowDown)
        ),
        Some(KeyCode::ArrowDown)
    );
}
