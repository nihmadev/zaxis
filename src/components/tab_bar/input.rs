//! Keyboard navigation of the tabs (roving focus) and the rule that picks the tab to
//! select when the active one is closed.

use std::collections::HashSet;

use winit::keyboard::{KeyCode, ModifiersState};

/// Where the pressed keys move the focus from `from`, skipping disabled tabs. The arrows
/// move one tab, Home and End jump to the ends; nothing wraps. Ctrl+Shift with an arrow
/// moves the tab itself ([`step`]), so it does not navigate. `None` when no navigation key
/// was pressed or there is nowhere to go.
pub(super) fn target(
    keys: &HashSet<KeyCode>,
    modifiers: ModifiersState,
    from: usize,
    enabled: &[bool],
) -> Option<usize> {
    if modifiers.alt_key() || (modifiers.control_key() && modifiers.shift_key()) {
        return None;
    }
    let first = || enabled.iter().position(|e| *e);
    let last = || enabled.iter().rposition(|e| *e);
    let (prev, next) = (KeyCode::ArrowLeft, KeyCode::ArrowRight);
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

/// Ctrl+Shift with an arrow key: `-1` moves the focused tab one place left, `1` right.
pub(super) fn step(keys: &HashSet<KeyCode>, modifiers: ModifiersState) -> Option<i32> {
    if modifiers.alt_key() || !(modifiers.control_key() && modifiers.shift_key()) {
        return None;
    }
    match (
        keys.contains(&KeyCode::ArrowLeft),
        keys.contains(&KeyCode::ArrowRight),
    ) {
        (true, false) => Some(-1),
        (false, true) => Some(1),
        _ => None,
    }
}

/// The tab to select after the tab at index `closed` goes away, given which tabs of the
/// row (before the removal) are enabled: the nearest enabled tab to its right, otherwise
/// the nearest enabled one to its left, skipping disabled tabs. `None` when no enabled tab
/// is left. Indices refer to the row before the removal.
///
/// ```
/// # use zaxis::tab_after_close;
/// assert_eq!(tab_after_close(1, &[true, true, true]), Some(2));
/// assert_eq!(tab_after_close(2, &[true, true, true]), Some(1));
/// assert_eq!(tab_after_close(0, &[true, false, true]), Some(2));
/// assert_eq!(tab_after_close(1, &[false, true, false]), None);
/// ```
pub fn tab_after_close(closed: usize, enabled: &[bool]) -> Option<usize> {
    let right = enabled
        .iter()
        .enumerate()
        .skip(closed + 1)
        .find_map(|(i, e)| e.then_some(i));
    right.or_else(|| {
        enabled[..closed.min(enabled.len())]
            .iter()
            .rposition(|e| *e)
    })
}
