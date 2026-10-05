//! Text fields: keys, committed and composed text, focus changes and pointer presses
//! routed to the field they belong to, the click count of a press sequence, and the
//! retained state of every field. Pointer positions are delivered in the field's own
//! coordinates, through the transform the last pass published for it.

use super::{gesture::ClickCounter, Context, EventResponse, HitAction, Id};
use crate::{components::text_edit::TextEditState, Vec2};
use std::collections::{HashMap, HashSet};
use winit::keyboard::{KeyCode, ModifiersState};

pub(crate) enum TextEditInput {
    Text(String),
    Commit(String),
    Key(KeyCode, ModifiersState),
    // Count is 1..=3 for presses, zero for captured dragging.
    Pointer(Vec2, bool, u8),
    Focus(bool),
    Preedit(String, Option<(usize, usize)>),
    /// From assistive technology: replace the whole text.
    SetValue(String),
    /// From assistive technology: select from the first byte offset to the second.
    Select(usize, usize),
}

#[derive(Default)]
pub(crate) struct TextFields {
    /// Buffer, history and scroll of each field, by field id.
    pub(crate) states: HashMap<Id, TextEditState>,
    /// Input waiting for each field's next pass.
    queue: HashMap<Id, Vec<TextEditInput>>,
    /// Text areas that take Tab as input: registered this pass, and on the last one.
    tabs: HashSet<Id>,
    tabs_previous: HashSet<Id>,
    /// Presses on one field: single, double (word) and triple (paragraph) clicks.
    pub(super) clicks: ClickCounter,
    /// A composition is in progress: Escape and Enter belong to the input method.
    pub(super) composing: bool,
}

impl TextFields {
    pub(super) fn push(&mut self, id: Id, input: TextEditInput) {
        self.queue.entry(id).or_default().push(input);
    }

    /// Focus moved from `lost` to `gained`: both fields hear of it.
    pub(super) fn focus_changed(&mut self, lost: Option<Id>, gained: Option<Id>) {
        for (id, gained) in [(lost, false), (gained, true)] {
            if let Some(id) = id {
                self.push(id, TextEditInput::Focus(gained));
            }
        }
        self.clicks.reset();
        self.composing = false;
    }

    /// The text area `id` takes Tab as input on the next pass.
    pub(crate) fn take_tab(&mut self, id: Id) {
        self.tabs.insert(id);
    }

    /// The Tab registrations of this pass decide the next input.
    pub(super) fn publish_tabs(&mut self) {
        self.tabs_previous = std::mem::take(&mut self.tabs);
    }

    /// Input nobody took this pass, including focus changes settled at its end, is
    /// dropped.
    pub(super) fn clear_queue(&mut self) {
        self.queue.clear();
    }

    /// State of fields whose body was not painted in this pass is dropped.
    pub(super) fn retire(&mut self, painted: impl Fn(Id) -> bool) {
        self.states
            .retain(|id, _| painted(crate::components::text_edit::body_id(*id)));
    }
}

impl Context {
    /// Deliver committed text from a custom input bridge (also used by native events).
    pub fn on_text_event(&mut self, text: &str) -> EventResponse {
        self.on_committed_text(text, false)
    }

    /// Typed (`ime` false) or composed text goes to the focused field.
    pub(super) fn on_committed_text(&mut self, text: &str, ime: bool) -> EventResponse {
        self.input.text.push_str(text);
        let target = self.interaction.focused_as(HitAction::TextEdit);
        if let Some(id) = target {
            self.text_fields.push(
                id,
                if ime {
                    TextEditInput::Commit(text.to_owned())
                } else {
                    TextEditInput::Text(text.to_owned())
                },
            );
        }
        self.request_repaint();
        EventResponse {
            consumed: target.is_some(),
            repaint: true,
        }
    }

    /// A composition changed. Returns whether a focused field takes it.
    pub(super) fn ime_preedit(&mut self, text: &str, cursor: Option<(usize, usize)>) -> bool {
        self.text_fields.composing = !text.is_empty();
        let Some(id) = self.interaction.focused_as(HitAction::TextEdit) else {
            return false;
        };
        self.text_fields
            .push(id, TextEditInput::Preedit(text.to_owned(), cursor));
        true
    }

    /// The input method went away: the focused widget drops its composition.
    pub(super) fn ime_disabled(&mut self) {
        self.text_fields.composing = false;
        if let Some(id) = self.interaction.focused {
            self.text_fields
                .push(id, TextEditInput::Preedit(String::new(), None));
        }
    }

    /// A key while a field has focus. Every key but Tab is the field's; Tab is too in a
    /// text area that takes it, unless Shift asks for focus traversal. Returns whether
    /// the key was consumed.
    pub(super) fn text_key(&mut self, id: Id, code: KeyCode, pressed: bool) -> bool {
        let tab_input =
            self.text_fields.tabs_previous.contains(&id) && !self.input.modifiers.shift_key();
        if code == KeyCode::Tab && !tab_input {
            return false;
        }
        self.input.record_key(code, pressed);
        if pressed {
            self.text_fields
                .push(id, TextEditInput::Key(code, self.input.modifiers));
        }
        true
    }

    /// A press captured by field `id`: one, two or three clicks in a row.
    pub(super) fn text_press(&mut self, id: Id, pointer: Vec2) {
        let shift = self.input.modifiers.shift_key();
        let now = crate::time::Instant::now();
        let count = self.text_fields.clicks.count(id, pointer, now, 3, !shift);
        self.text_fields
            .push(id, TextEditInput::Pointer(pointer, shift, count));
    }

    /// The pointer moved while field `id` holds the capture: extend the selection.
    pub(super) fn text_drag(&mut self, id: Id, pointer: Vec2) {
        self.text_fields.clicks.moved(pointer);
        self.text_fields
            .push(id, TextEditInput::Pointer(pointer, true, 0));
    }

    /// Pointer position in the field's own (untransformed) coordinates, e.g. for drag
    /// auto-scroll.
    pub(crate) fn text_edit_pointer(&self, id: Id) -> Option<Vec2> {
        let transform = self.visuals.to_local(id);
        self.input.pointer.map(|p| transform.point(p))
    }

    pub(crate) fn take_text_edit_input(&mut self, id: Id) -> Vec<TextEditInput> {
        let transform = self.visuals.to_local(id);
        self.text_fields
            .queue
            .remove(&id)
            .unwrap_or_default()
            .into_iter()
            .map(|input| match input {
                TextEditInput::Pointer(p, extend, count) => {
                    TextEditInput::Pointer(transform.point(p), extend, count)
                }
                input => input,
            })
            .collect()
    }
}
