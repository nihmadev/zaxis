//! A numeric control edited as text runs the shared TextEdit (selection, clipboard, IME,
//! history and geometry are its own). This adapter only turns its events into numeric
//! transitions: Enter commits, Escape restores the snapshot, Up/Down commit and step,
//! focus loss commits a valid draft, and any other edit keeps the draft as typed.

use super::{
    access,
    state::{arrow, commit, multiplier},
    NumberOptions, NumberState, NumberStyle, Numeric,
};
use crate::{context::TextEditInput, Id, Response, TextEdit, Ui, Widget};
use winit::keyboard::KeyCode;

/// What the edit of this pass ended with.
#[derive(Clone, Copy, Default)]
pub(super) struct Outcome {
    pub submitted: bool,
    pub cancelled: bool,
}

struct Adapter<'s, 'o, T: Numeric> {
    state: &'s mut NumberState,
    value: &'s mut T,
    options: &'s NumberOptions<'o, T>,
    style: &'s NumberStyle,
    /// A drag value: the edit ends with its commit or cancel.
    drag: bool,
    outcome: Outcome,
}

impl<T: Numeric> Adapter<'_, '_, T> {
    /// Handle one event of the field; true when the TextEdit must not handle it too.
    fn handle(&mut self, buffer: &mut String, event: &TextEditInput) -> bool {
        let (state, value) = (&mut *self.state, &mut *self.value);
        let (options, style, drag) = (self.options, self.style, self.drag);
        let outcome = &mut self.outcome;
        match event {
            TextEditInput::Focus(true) if !state.editing => {
                state.begin(*value);
                buffer.clone_from(&state.buffer);
                false
            }
            TextEditInput::Focus(false) => {
                if state.editing {
                    commit(buffer, value, options);
                }
                state.editing = false;
                state.invalid = false;
                *buffer = options.display(*value);
                false
            }
            TextEditInput::Key(KeyCode::Enter | KeyCode::NumpadEnter, _) => {
                if commit(buffer, value, options) {
                    outcome.submitted = true;
                    state.committed(*value);
                    *buffer = value.to_string();
                    state.editing &= !drag;
                } else {
                    state.invalid = true;
                }
                true
            }
            TextEditInput::Key(KeyCode::Escape, _) => {
                state.cancel(value);
                *buffer = value.to_string();
                outcome.cancelled = true;
                state.editing &= !drag;
                true
            }
            TextEditInput::Key(key @ (KeyCode::ArrowUp | KeyCode::ArrowDown), mods) => {
                // Incomplete drafts must not turn into zero on a step.
                if commit(buffer, value, options) {
                    let units = arrow(*key) * multiplier(*mods, style);
                    state.step_from(value, options, units);
                    *buffer = value.to_string();
                } else {
                    state.invalid = true;
                }
                true
            }
            TextEditInput::SetValue(text) => {
                // Assistive technology set the field's text: that is a typed commit.
                if access::set_text(state, value, options, text) {
                    *buffer = if state.editing {
                        value.to_string()
                    } else {
                        options.display(*value)
                    };
                }
                true
            }
            _ if drag && !state.editing => true,
            _ => {
                state.invalid = false;
                false
            }
        }
    }
}

/// Build the field of control `id`: its draft while editing, the displayed value
/// otherwise. `began` selects everything, as when an edit has just started.
pub(super) fn show<T: Numeric>(
    ui: &mut Ui<'_>,
    id: Id,
    (state, value): (&mut NumberState, &mut T),
    options: &NumberOptions<'_, T>,
    style: &NumberStyle,
    (drag, began): (bool, bool),
) -> (Response, Outcome) {
    let mut buffer = std::mem::take(&mut state.buffer);
    if !state.editing {
        buffer = options.display(*value);
    }
    let (response, outcome) = {
        let mut adapter = Adapter {
            state: &mut *state,
            value,
            options,
            style,
            drag,
            outcome: Outcome::default(),
        };
        let mut handler =
            |buffer: &mut String, event: &TextEditInput| adapter.handle(buffer, event);
        let mut editor = TextEdit::new(&mut buffer)
            .enabled(options.enabled)
            .width(style.width)
            .height(style.height)
            .padding(style.padding)
            .corner_radius(style.rounding)
            .font_size(style.font_size)
            .status(options.status)
            .style(crate::TextEditStyle {
                surface: style.surface,
                ..Default::default()
            });
        if let Some(hover) = options.hover_style {
            editor = editor.hover_style(hover);
        }
        editor.exact_id = Some(id);
        editor.select_all = began;
        editor.affixes = (options.prefix.clone(), options.suffix.clone());
        editor.event_handler = Some(&mut handler);
        editor = editor.fill(style.fill);
        editor.hovered_fill = Some(style.hovered);
        let response = editor.ui(ui);
        (response, adapter.outcome)
    };
    state.buffer = buffer;
    if state.editing && T::parse(&state.buffer).is_none() {
        state.invalid = true;
    }
    (response, outcome)
}
