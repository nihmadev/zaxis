use super::{
    number_input::{number_builders, show_drag, NumberOptions},
    NumberStyle, Numeric, Response, Ui, Widget,
};
use crate::Id;
use std::{hash::Hash, ops::RangeInclusive, panic::Location};

/// Drag horizontally to adjust. Click without moving, Enter or F2 starts TextEdit.
/// Arrows adjust; Shift is precise, Ctrl accelerated. See [`super::NumberInput`]
/// for commit, cancellation and focus-loss rules.
pub struct DragValue<'a, T: Numeric> {
    value: &'a mut T,
    options: NumberOptions<'a, T>,
}
impl<'a, T: Numeric> DragValue<'a, T> {
    #[track_caller]
    pub fn new(value: &'a mut T) -> Self {
        Self {
            value,
            options: NumberOptions::new(Location::caller()),
        }
    }
    number_builders!();
}
impl<T: Numeric> Widget for DragValue<'_, T> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        show_drag(ui, self.value, self.options)
    }
}
impl Ui<'_> {
    #[track_caller]
    pub fn drag_value<T: Numeric>(&mut self, value: &mut T) -> Response {
        self.add(DragValue::new(value))
    }
}
