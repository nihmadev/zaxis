//! Value controls: pointer and key input routed to sliders (palette and hue surfaces of a
//! color picker included) and drag values, and the retained state of numeric and color
//! editors. Pointer positions are delivered in the control's own coordinates.

use super::{Context, Id};
use crate::{
    components::{color_picker::ColorPickerState, number_input::NumberState},
    Vec2,
};
use std::collections::HashMap;
use winit::keyboard::{KeyCode, ModifiersState};

#[derive(Clone, Copy)]
pub enum SliderInput {
    Pointer(Vec2),
    Key(KeyCode),
    /// A value set directly, by assistive technology; normalized like any other input.
    Set(f64),
}

/// Input of a drag value, with the modifiers held at the time.
pub(crate) enum NumberInputEvent {
    /// The primary button went down on it.
    Press(Vec2, ModifiersState),
    /// The pointer moved while it held the capture.
    Drag(Vec2, ModifiersState),
    /// The primary button was released while it held the capture.
    Release(Vec2, ModifiersState),
    Key(KeyCode, ModifiersState),
}

#[derive(Default)]
pub(crate) struct ValueControls {
    /// Draft, snapshot and drag state of each numeric control.
    pub(crate) numbers: HashMap<Id, NumberState>,
    /// Hue and open state of each color picker.
    pub(crate) color_pickers: HashMap<Id, ColorPickerState>,
    numbers_input: HashMap<Id, Vec<NumberInputEvent>>,
    pub(super) sliders_input: HashMap<Id, Vec<SliderInput>>,
}

impl ValueControls {
    /// Input nobody took this pass is dropped.
    pub(super) fn clear_queues(&mut self) {
        self.sliders_input.clear();
        self.numbers_input.clear();
    }

    /// Numeric controls not built in pass `frame` lose their state.
    pub(super) fn retire_numbers(&mut self, frame: u64) {
        self.numbers.retain(|_, state| state.last_frame == frame);
    }

    /// Color pickers whose swatch was not painted in this pass lose their state.
    pub(super) fn retire_color_pickers(&mut self, painted: impl Fn(Id) -> bool) {
        self.color_pickers
            .retain(|id, _| painted(crate::components::color_picker::swatch_id(*id)));
    }
}

/// Keys that move a slider or a drag value.
fn adjusting(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::PageUp
            | KeyCode::PageDown
    )
}

impl Context {
    /// A key while a drag value has focus: adjustment keys, Enter or F2 to edit it as text,
    /// Escape to cancel a drag. Returns whether it was consumed.
    pub(super) fn drag_value_key(&mut self, id: Id, code: KeyCode, pressed: bool) -> bool {
        if !adjusting(code)
            && !matches!(
                code,
                KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::F2 | KeyCode::Escape
            )
        {
            return false;
        }
        self.input.record_key(code, pressed);
        if pressed {
            let event = NumberInputEvent::Key(code, self.input.modifiers);
            self.values.numbers_input.entry(id).or_default().push(event);
        }
        true
    }

    /// A key while a slider has focus. Returns whether it was consumed.
    pub(super) fn slider_key(&mut self, id: Id, code: KeyCode, pressed: bool) -> bool {
        if !adjusting(code) {
            return false;
        }
        self.input.record_key(code, pressed);
        if pressed {
            self.slider_pointer_or_key(id, SliderInput::Key(code));
        }
        true
    }

    /// The pointer pressed, dragged or released on slider `id`.
    pub(super) fn slider_pointer(&mut self, id: Id, pointer: Vec2) {
        self.slider_pointer_or_key(id, SliderInput::Pointer(pointer));
    }

    fn slider_pointer_or_key(&mut self, id: Id, input: SliderInput) {
        self.values.sliders_input.entry(id).or_default().push(input);
    }

    /// Pointer input of drag value `id`.
    pub(super) fn drag_value_pointer(&mut self, id: Id, event: NumberInputEvent) {
        self.values.numbers_input.entry(id).or_default().push(event);
    }

    pub(crate) fn take_slider_input(&mut self, id: Id) -> Vec<SliderInput> {
        let transform = self.visuals.to_local(id);
        self.values
            .sliders_input
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|input| match input {
                SliderInput::Pointer(p) => SliderInput::Pointer(transform.point(p)),
                input => input,
            })
            .collect()
    }

    pub(crate) fn take_number_input(&mut self, id: Id) -> Vec<NumberInputEvent> {
        let transform = self.visuals.to_local(id);
        self.values
            .numbers_input
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|event| match event {
                NumberInputEvent::Press(p, mods) => {
                    NumberInputEvent::Press(transform.point(p), mods)
                }
                NumberInputEvent::Drag(p, mods) => NumberInputEvent::Drag(transform.point(p), mods),
                NumberInputEvent::Release(p, mods) => {
                    NumberInputEvent::Release(transform.point(p), mods)
                }
                event => event,
            })
            .collect()
    }
}
