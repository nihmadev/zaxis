use std::collections::HashSet;

use winit::keyboard::KeyCode;

use zaxis::components::radio::input::{columns, target};
use zaxis::components::radio::options::RadioLayout;

fn keys(code: KeyCode) -> HashSet<KeyCode> {
    HashSet::from([code])
}

#[test]
fn arrows_wrap_and_skip_disabled() {
    let enabled = [true, false, true];
    let v = RadioLayout::Vertical;
    assert_eq!(target(&keys(KeyCode::ArrowDown), v, 0, &enabled), Some(2));
    assert_eq!(target(&keys(KeyCode::ArrowDown), v, 2, &enabled), Some(0));
    assert_eq!(target(&keys(KeyCode::ArrowUp), v, 0, &enabled), Some(2));
    assert_eq!(target(&keys(KeyCode::ArrowLeft), v, 0, &enabled), None);
    assert_eq!(target(&keys(KeyCode::End), v, 0, &enabled), Some(2));
    assert_eq!(target(&keys(KeyCode::Home), v, 2, &enabled), Some(0));
}

#[test]
fn single_enabled_option_has_nowhere_to_go() {
    let enabled = [false, true];
    let v = RadioLayout::Vertical;
    assert_eq!(target(&keys(KeyCode::ArrowDown), v, 1, &enabled), None);
    assert_eq!(target(&keys(KeyCode::Home), v, 1, &enabled), None);
}

#[test]
fn grid_columns_are_clamped_and_down_stays_in_the_column() {
    assert_eq!(columns(RadioLayout::Grid(9), 4), 4);
    assert_eq!(columns(RadioLayout::Grid(0), 4), 1);
    let enabled = [true; 5];
    let g = RadioLayout::Grid(2);
    assert_eq!(target(&keys(KeyCode::ArrowDown), g, 0, &enabled), Some(2));
    assert_eq!(target(&keys(KeyCode::ArrowDown), g, 4, &enabled), Some(0));
    assert_eq!(target(&keys(KeyCode::ArrowDown), g, 1, &enabled), Some(3));
    assert_eq!(target(&keys(KeyCode::ArrowRight), g, 4, &enabled), Some(0));
}
