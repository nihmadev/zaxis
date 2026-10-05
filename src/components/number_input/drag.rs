//! A drag value that is not being edited: horizontal pointer drags step it (past a small
//! threshold, without accumulating the dead travel), a click without motion, Enter or F2
//! start a text edit, arrows and pages step, Home and End jump to the bounds, and Escape
//! cancels a drag in progress.

use super::{
    state::{arrow, multiplier},
    NumberOptions, NumberState, NumberStyle, Numeric,
};
use crate::context::NumberInputEvent;
use winit::keyboard::KeyCode;

/// Pointer travel, in logical pixels, before a press becomes a drag.
const THRESHOLD: f32 = 3.0;

/// Apply this pass's input of a drag value in arrival order.
pub(super) fn apply<T: Numeric>(
    state: &mut NumberState,
    value: &mut T,
    options: &NumberOptions<'_, T>,
    style: &NumberStyle,
    events: Vec<NumberInputEvent>,
) {
    for event in events {
        match event {
            NumberInputEvent::Press(p, _) => {
                state.pointer = Some(p);
                state.distance = 0.0;
                state.rebase(*value);
                state.snapshot = value.to_string();
            }
            NumberInputEvent::Drag(p, mods) => moved(state, value, options, style, p, mods),
            NumberInputEvent::Release(p, mods) => {
                if state.pointer.is_some() {
                    moved(state, value, options, style, p, mods);
                    state.pointer = None;
                    if state.distance <= THRESHOLD {
                        state.begin(*value);
                    }
                }
            }
            NumberInputEvent::Key(KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::F2, _) => {
                state.begin(*value)
            }
            NumberInputEvent::Key(KeyCode::Escape, _) => {
                if state.pointer.take().is_some() {
                    if let Ok(original) = state.snapshot.parse::<T>() {
                        *value = original;
                    }
                    state.rebase(*value);
                }
            }
            NumberInputEvent::Key(key, mods) => {
                let units = arrow(key) * multiplier(mods, style);
                if units != 0.0 {
                    state.step(value, options, units);
                } else if matches!(key, KeyCode::Home | KeyCode::End) {
                    *value = if key == KeyCode::Home {
                        *options.range.start()
                    } else {
                        *options.range.end()
                    };
                    state.rebase(*value);
                }
            }
        }
    }
}

/// The pressed pointer moved to `pointer`. Horizontal motion counts once the travel passes
/// the threshold; the part of the first motion inside it is dead travel.
fn moved<T: Numeric>(
    state: &mut NumberState,
    value: &mut T,
    options: &NumberOptions<'_, T>,
    style: &NumberStyle,
    pointer: crate::Vec2,
    mods: winit::keyboard::ModifiersState,
) {
    let Some(previous) = state.pointer else {
        return;
    };
    let dx = pointer.x - previous.x;
    let prior = state.distance;
    state.distance += dx.abs();
    state.pointer = Some(pointer);
    if state.distance > THRESHOLD {
        let motion = if prior <= THRESHOLD {
            state.distance - THRESHOLD
        } else {
            dx.abs()
        };
        let units = f64::from(motion * dx.signum()) * style.sensitivity * multiplier(mods, style);
        state.step(value, options, units);
    }
}
