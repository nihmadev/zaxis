//! Keyboard navigation between the options (roving focus).

use std::collections::HashSet;

use winit::keyboard::KeyCode;

use super::options::RadioLayout;

/// Columns of the arrangement for `count` options.
pub fn columns(layout: RadioLayout, count: usize) -> usize {
    match layout {
        RadioLayout::Vertical => 1,
        RadioLayout::Horizontal => count.max(1),
        RadioLayout::Grid(n) => n.clamp(1, count.max(1)),
    }
}

/// First enabled index after (or before) `from` in `seq`, walking around the end.
fn cycle(seq: &[usize], from: usize, forward: bool, enabled: &[bool]) -> Option<usize> {
    let at = seq.iter().position(|i| *i == from)?;
    (1..seq.len())
        .map(|step| {
            let k = if forward {
                (at + step) % seq.len()
            } else {
                (at + seq.len() - step) % seq.len()
            };
            seq[k]
        })
        .find(|i| enabled[*i])
}

/// Where the pressed keys move the focus from `from`, skipping disabled options. Arrows follow
/// the layout and wrap around (in a grid, Up/Down stay in their column), Home/End jump to the
/// ends. `None` when no navigation key was pressed or there is nowhere to go.
pub fn target(
    keys: &HashSet<KeyCode>,
    layout: RadioLayout,
    from: usize,
    enabled: &[bool],
) -> Option<usize> {
    let n = enabled.len();
    if from >= n {
        return None;
    }
    let cols = columns(layout, n);
    let flat: Vec<usize> = (0..n).collect();
    let column: Vec<usize> = (from % cols..n).step_by(cols).collect();
    let pressed = |a: KeyCode, b: KeyCode| match (keys.contains(&a), keys.contains(&b)) {
        (true, false) => Some(false),
        (false, true) => Some(true),
        _ => None,
    };
    let across = pressed(KeyCode::ArrowLeft, KeyCode::ArrowRight);
    let down = pressed(KeyCode::ArrowUp, KeyCode::ArrowDown);
    let to = if keys.contains(&KeyCode::Home) {
        enabled.iter().position(|e| *e)
    } else if keys.contains(&KeyCode::End) {
        enabled.iter().rposition(|e| *e)
    } else {
        match layout {
            RadioLayout::Vertical => down.and_then(|f| cycle(&flat, from, f, enabled)),
            RadioLayout::Horizontal => across.and_then(|f| cycle(&flat, from, f, enabled)),
            RadioLayout::Grid(_) => across
                .and_then(|f| cycle(&flat, from, f, enabled))
                .or_else(|| down.and_then(|f| cycle(&column, from, f, enabled))),
        }
    };
    to.filter(|to| *to != from)
}
