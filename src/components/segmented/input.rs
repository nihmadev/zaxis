//! Keyboard navigation of the segments (roving focus).

use std::collections::HashSet;

use winit::keyboard::KeyCode;

/// Where the pressed keys move the focus from `from`, skipping disabled segments. Arrows
/// follow the orientation, Home/End jump to the ends; nothing wraps. `None` when no
/// navigation key was pressed or there is nowhere to go.
pub(super) fn target(
    keys: &HashSet<KeyCode>,
    vertical: bool,
    from: usize,
    enabled: &[bool],
) -> Option<usize> {
    let (prev, next) = if vertical {
        (KeyCode::ArrowUp, KeyCode::ArrowDown)
    } else {
        (KeyCode::ArrowLeft, KeyCode::ArrowRight)
    };
    let first = || enabled.iter().position(|e| *e);
    let last = || enabled.iter().rposition(|e| *e);
    let to = if keys.contains(&KeyCode::Home) {
        first()
    } else if keys.contains(&KeyCode::End) {
        last()
    } else if keys.contains(&prev) && !keys.contains(&next) {
        enabled[..from.min(enabled.len())].iter().rposition(|e| *e)
    } else if keys.contains(&next) && !keys.contains(&prev) {
        enabled
            .iter()
            .enumerate()
            .skip(from + 1)
            .find_map(|(i, e)| e.then_some(i))
    } else {
        None
    };
    to.filter(|to| *to != from)
}
