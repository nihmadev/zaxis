//! Typed numeric operations and the state transitions of one numeric control. Values are
//! kept as their exact text (`to_string`), so integers never pass through floating point
//! and every transition leaves draft, snapshot and step origin consistent.

use super::{NumberOptions, NumberStyle, Numeric};
use crate::Vec2;
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Default)]
pub(crate) struct NumberState {
    /// The text being edited; meaningful while `editing`.
    pub(super) buffer: String,
    /// The value Escape returns to: where the edit or drag began, or the last commit.
    pub(super) snapshot: String,
    /// The value seen at the end of the last pass, to notice changes made outside.
    pub(super) last_value: String,
    pub(super) editing: bool,
    /// Steps count from this value; `units` steps have been taken from it.
    origin: String,
    units: f64,
    /// The step the units were counted in.
    last_step: String,
    /// A drag value's pressed pointer and how far it moved.
    pub(super) pointer: Option<Vec2>,
    pub(super) distance: f32,
    /// Focus at the end of the last pass, for `lost_focus`.
    pub(super) focused: bool,
    /// The draft is not a number; shown as an invalid border.
    pub(super) invalid: bool,
    pub(crate) last_frame: u64,
}

impl NumberState {
    /// `value` was accepted from outside the typed text, or set while the control was
    /// idle: no draft is pending, and Escape and steps start from it.
    pub(super) fn committed<T: Numeric>(&mut self, value: T) {
        self.invalid = false;
        self.snapshot = value.to_string();
        self.origin = self.snapshot.clone();
        self.units = 0.0;
        self.buffer = self.snapshot.clone();
    }

    /// The value changed outside the control, or the control is disabled; a disabled
    /// control also ends its edit and drag.
    pub(super) fn observe<T: Numeric>(&mut self, value: T, enabled: bool) {
        if self.last_value != value.to_string() || !enabled {
            self.committed(value);
        }
        if !enabled {
            self.editing = false;
            self.pointer = None;
        }
    }

    /// Start editing `value` as text.
    pub(super) fn begin<T: Numeric>(&mut self, value: T) {
        self.committed(value);
        self.editing = true;
    }

    /// Escape: back to the snapshot, which stays the value Escape returns to.
    pub(super) fn cancel<T: Numeric>(&mut self, value: &mut T) {
        if let Ok(original) = self.snapshot.parse::<T>() {
            *value = original;
        }
        self.committed(*value);
    }

    /// Steps restart from `value`: after a jump to a bound or a cancelled drag.
    pub(super) fn rebase<T: Numeric>(&mut self, value: T) {
        self.origin = value.to_string();
        self.units = 0.0;
    }

    /// Move `units` steps (fractional for drags) from the origin, within the range. A
    /// bound that stops the motion becomes the new origin, so reversing responds at once.
    pub(super) fn step<T: Numeric>(
        &mut self,
        value: &mut T,
        options: &NumberOptions<'_, T>,
        units: f64,
    ) {
        let (min, max) = (*options.range.start(), *options.range.end());
        if self.origin.is_empty() || self.last_step != options.step.to_string() {
            self.rebase(*value);
        }
        let origin = self.origin.parse::<T>().unwrap_or(*value);
        self.last_step = options.step.to_string();
        self.units += units;
        *value = origin.offset(options.step, self.units, min, max);
        if (*value == min && units < 0.0) || (*value == max && units > 0.0) {
            self.rebase(*value);
        }
        self.invalid = false;
    }

    /// Step from the value as it is now: a value that is not where the last steps left it
    /// (it was typed, committed or set meanwhile) becomes the new origin.
    pub(super) fn step_from<T: Numeric>(
        &mut self,
        value: &mut T,
        options: &NumberOptions<'_, T>,
        units: f64,
    ) {
        let (min, max) = (*options.range.start(), *options.range.end());
        let origin = self.origin.parse::<T>().ok();
        if origin.is_none_or(|o| !o.offset(options.step, self.units, min, max).same(*value)) {
            self.rebase(*value);
        }
        self.step(value, options, units);
    }
}

/// Parse `text` and store it clamped to the range. Text that is no number leaves `value`
/// unchanged and returns false.
pub(super) fn commit<T: Numeric>(
    text: &str,
    value: &mut T,
    options: &NumberOptions<'_, T>,
) -> bool {
    let Some(next) = T::parse(text) else {
        return false;
    };
    *value = next.normalized(*options.range.start(), *options.range.end());
    true
}

/// Shift is precise, Ctrl accelerated.
pub(super) fn multiplier(mods: ModifiersState, style: &NumberStyle) -> f64 {
    if mods.shift_key() {
        style.shift_multiplier
    } else if mods.control_key() {
        style.ctrl_multiplier
    } else {
        1.0
    }
}

/// Steps of an arrow or page key; zero for any other key.
pub(super) fn arrow(key: KeyCode) -> f64 {
    match key {
        KeyCode::ArrowUp | KeyCode::ArrowRight => 1.0,
        KeyCode::ArrowDown | KeyCode::ArrowLeft => -1.0,
        KeyCode::PageUp => 10.0,
        KeyCode::PageDown => -10.0,
        _ => 0.0,
    }
}
