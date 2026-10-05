//! Input handling shared by single- and multi-line fields. Layout-dependent questions are
//! answered by a [`Surface`]; everything else (selection, clipboard, undo, IME) is common.
use super::{
    doc::Doc,
    doc_layout::AreaSurface,
    single::LineSurface,
    text_input::{fit, multi_line, single_line},
    *,
};
use crate::{
    components::edit_buffer::{paragraph_at, snap_down, word_at, Change, Delta, Surface},
    context::Context,
};

/// What one pass of input produced.
#[derive(Default)]
pub(super) struct Outcome {
    pub submitted: bool,
    pub lost_focus: bool,
    pub changed: bool,
    /// The caret or text moved by keyboard, IME or undo: the caret should stay visible.
    pub reveal: bool,
    /// Pages scrolled with PageUp/PageDown, in visual lines (signed).
    pub scroll_lines: i32,
    pub deltas: Vec<Delta>,
}

/// Layout context for event handling.
pub(super) struct Geo<'a> {
    /// The paragraph table of a multi-line field.
    pub doc: Option<&'a mut Doc>,
    pub size: f32,
    pub font: crate::text::TextFont,
    /// A pointer position minus this is a position in content coordinates.
    pub origin: Vec2,
    pub page: i32,
}

fn surface<R>(geo: &mut Geo<'_>, ctx: &mut Context, f: impl FnOnce(&mut dyn Surface) -> R) -> R {
    match geo.doc.as_deref_mut() {
        Some(doc) => f(&mut AreaSurface {
            doc,
            ctx,
            page: geo.page,
        }),
        None => f(&mut LineSurface {
            ctx,
            size: geo.size,
            font: geo.font,
        }),
    }
}

impl TextEdit<'_> {
    pub(super) fn process_events(
        &mut self,
        ui: &mut Ui<'_>,
        state: &mut TextEditState,
        out: &mut Outcome,
        events: Vec<TextEditInput>,
        geo: &mut Geo<'_>,
    ) {
        // The press that focuses a whole-value field keeps its whole-text selection.
        let mut focusing = false;
        for event in events {
            if self.run_handler(state, geo, out, &event) {
                continue;
            }
            let before = state.buffer.clone();
            let typing = matches!(&event, TextEditInput::Text(text)
                if state.preedit.is_none() && before.selection().is_empty()
                    && text.graphemes(true).count() == 1 && !text.chars().any(char::is_control));
            let interrupts_typing = match &event {
                TextEditInput::Key(key, mods) => {
                    mods.control_key()
                        || mods.super_key()
                        || matches!(
                            key,
                            KeyCode::ArrowLeft
                                | KeyCode::ArrowRight
                                | KeyCode::ArrowUp
                                | KeyCode::ArrowDown
                                | KeyCode::Home
                                | KeyCode::End
                                | KeyCode::PageUp
                                | KeyCode::PageDown
                                | KeyCode::Backspace
                                | KeyCode::Delete
                                | KeyCode::Enter
                                | KeyCode::NumpadEnter
                                | KeyCode::Tab
                                | KeyCode::Escape
                        )
                }
                _ => !typing,
            };
            if interrupts_typing {
                state.history.break_group();
            }
            let keyboard = !matches!(event, TextEditInput::Pointer(..) | TextEditInput::Focus(_));
            let mut change = None;
            match event {
                TextEditInput::Focus(false) => {
                    out.lost_focus = true;
                    state.preedit = None;
                }
                TextEditInput::Focus(true) => {
                    if self.whole_value {
                        state.buffer.select_all(self.text);
                        focusing = true;
                    }
                }
                _ if !self.enabled => {}
                TextEditInput::Text(text) | TextEditInput::Commit(text) if !self.read_only => {
                    state.preedit = None;
                    change = self.insert(state, geo, &text);
                }
                TextEditInput::Preedit(text, cursor) if !self.read_only => {
                    if geo.doc.is_some() && !text.is_empty() {
                        // Composition replaces the selection from its first character.
                        change = state.buffer.replace_selection(self.text, "");
                    }
                    let shown: String = text.chars().filter(|c| !c.is_control()).collect();
                    state.preedit = (!text.is_empty()).then_some((shown, cursor));
                }
                TextEditInput::SetValue(text) if !self.read_only => {
                    state.preedit = None;
                    change = self.replace_all(state, geo, &text);
                }
                TextEditInput::Select(anchor, focus) if state.preedit.is_none() => {
                    let at = |byte: usize| snap_down(self.text, byte.min(self.text.len()));
                    (state.buffer.anchor, state.buffer.cursor) = (at(anchor), at(focus));
                    (state.buffer.upstream, state.buffer.column) = (false, None);
                    state.word_drag = None;
                }
                TextEditInput::Pointer(_, false, 1) if std::mem::take(&mut focusing) => {}
                TextEditInput::Pointer(pointer, extend, count) => {
                    self.pointer(ui, state, geo, pointer, extend, count);
                }
                TextEditInput::Key(key, modifiers) if state.preedit.is_none() => {
                    change = self.key(ui, state, geo, out, key, modifiers);
                }
                _ => {}
            }
            if let Some(change) = change {
                let delta = change.delta();
                state
                    .history
                    .record(change, before.clone(), state.buffer.clone(), typing);
                self.edited(state, geo, out, delta);
            }
            out.reveal |=
                keyboard && (out.changed || state.buffer != before || state.preedit.is_some());
        }
    }

    /// Run the owner's custom key handler (single-line helpers such as number fields).
    fn run_handler(
        &mut self,
        state: &mut TextEditState,
        geo: &mut Geo<'_>,
        out: &mut Outcome,
        event: &TextEditInput,
    ) -> bool {
        let live = self.enabled
            && (state.preedit.is_none()
                || matches!(
                    event,
                    TextEditInput::Focus(_) | TextEditInput::Key(KeyCode::Escape, _)
                ));
        let Some(handler) = self.event_handler.as_mut().filter(|_| live) else {
            return false;
        };
        let previous = self.text.clone();
        let handled = handler(self.text, event);
        if previous != *self.text {
            state.buffer.select_all(self.text);
            state.history = EditHistory::default();
            out.changed = true;
            if let Some(doc) = geo.doc.as_deref_mut() {
                doc.reset(self.text);
            }
        }
        if handled && matches!(event, TextEditInput::Key(KeyCode::Escape, _)) {
            state.preedit = None;
        }
        if handled
            && self.whole_value
            && matches!(
                event,
                TextEditInput::Key(KeyCode::Enter | KeyCode::NumpadEnter | KeyCode::Escape, _)
            )
        {
            state.buffer.select_all(self.text);
        }
        handled
    }

    /// Bookkeeping after the text changed through the field itself.
    fn edited(
        &mut self,
        state: &mut TextEditState,
        geo: &mut Geo<'_>,
        out: &mut Outcome,
        delta: Delta,
    ) {
        if let Some(doc) = geo.doc.as_deref_mut() {
            doc.apply(self.text, delta);
        }
        out.deltas.push(delta);
        out.changed = true;
        state.word_drag = None;
    }

    /// Insert typed, pasted or committed text, normalized for the mode and the length limit.
    fn insert(&mut self, state: &mut TextEditState, geo: &Geo<'_>, text: &str) -> Option<Change> {
        let value = if geo.doc.is_some() {
            multi_line(text)
        } else {
            single_line(text)
        };
        let value = self.accepted(value);
        let selection = state.buffer.selection();
        let value = fit(self.text, &self.text[selection], value, self.max_chars);
        if value.is_empty() {
            return None;
        }
        state.buffer.replace_selection(self.text, &value)
    }

    /// Replace the whole text, normalized and limited like typed text; one undo step.
    fn replace_all(
        &mut self,
        state: &mut TextEditState,
        geo: &Geo<'_>,
        text: &str,
    ) -> Option<Change> {
        let value = if geo.doc.is_some() {
            multi_line(text)
        } else {
            single_line(text)
        };
        let value = fit(self.text, self.text, self.accepted(value), self.max_chars);
        state.buffer.select_all(self.text);
        state.buffer.replace_selection(self.text, &value)
    }

    /// `value` without the characters the field does not take.
    fn accepted(&self, value: String) -> String {
        match self.accept {
            Some(accept) => value.chars().filter(|c| accept(*c)).collect(),
            None => value,
        }
    }

    pub(super) fn pointer(
        &mut self,
        ui: &mut Ui<'_>,
        state: &mut TextEditState,
        geo: &mut Geo<'_>,
        pointer: Vec2,
        extend: bool,
        count: u8,
    ) {
        state.preedit = None;
        let point = pointer - geo.origin;
        let multi = geo.doc.is_some();
        let (pos, cell) = surface(geo, ui.context, |s| s.hit(self.text, point));
        let paragraph = |text: &str| {
            if multi {
                paragraph_at(text, cell)
            } else {
                0..text.len()
            }
        };
        let buffer = &mut state.buffer;
        match count {
            2 | 3 => {
                let range = if count == 2 {
                    word_at(self.text, cell)
                } else {
                    paragraph(self.text)
                };
                (buffer.anchor, buffer.cursor) = (range.start, range.end);
                (buffer.upstream, buffer.column) = (false, None);
                state.drag_paragraphs = count == 3;
                state.word_drag = Some(range);
            }
            0 if state.word_drag.is_some() => {
                let anchor = state.word_drag.as_ref().expect("checked above");
                let unit = if state.drag_paragraphs {
                    paragraph(self.text)
                } else {
                    word_at(self.text, cell)
                };
                if unit.start < anchor.start {
                    (buffer.anchor, buffer.cursor) = (anchor.end, unit.start);
                } else {
                    (buffer.anchor, buffer.cursor) = (anchor.start, unit.end.max(anchor.end));
                }
            }
            _ => {
                state.word_drag = None;
                buffer.set_pos(pos, extend);
            }
        }
    }

    fn key(
        &mut self,
        ui: &mut Ui<'_>,
        state: &mut TextEditState,
        geo: &mut Geo<'_>,
        out: &mut Outcome,
        key: KeyCode,
        modifiers: winit::keyboard::ModifiersState,
    ) -> Option<Change> {
        let command = modifiers.control_key() || modifiers.super_key();
        let area = self.area.filter(|_| geo.doc.is_some());
        match key {
            KeyCode::KeyZ | KeyCode::KeyY if command && !self.read_only => {
                let redo = key == KeyCode::KeyY || modifiers.shift_key();
                if let Some(delta) = state.history.restore(self.text, &mut state.buffer, redo) {
                    self.edited(state, geo, out, delta);
                }
                None
            }
            KeyCode::Enter | KeyCode::NumpadEnter => match area {
                None => {
                    out.submitted = true;
                    None
                }
                Some(options) if command && options.submit_on_ctrl_enter => {
                    out.submitted = true;
                    None
                }
                Some(_) if !self.read_only => self.insert(state, geo, "\n"),
                Some(_) => None,
            },
            KeyCode::Tab if area.is_some_and(|a| a.tab_indent) && !self.read_only => {
                self.insert(state, geo, "\t")
            }
            KeyCode::KeyC | KeyCode::KeyX if command => {
                let range = state.buffer.selection();
                if !range.is_empty()
                    && ui.context.copy_text(self.text[range].to_owned()).is_ok()
                    && key == KeyCode::KeyX
                    && !self.read_only
                {
                    return state.buffer.replace_selection(self.text, "");
                }
                None
            }
            KeyCode::KeyV if command && !self.read_only => {
                let text = ui.context.clipboard_text().ok()?;
                self.insert(state, geo, &text)
            }
            _ => {
                if area.is_some() && !command && matches!(key, KeyCode::PageUp | KeyCode::PageDown)
                {
                    let page = geo.page;
                    out.scroll_lines += if key == KeyCode::PageUp { -page } else { page };
                }
                let (buffer, text) = (&mut state.buffer, &mut *self.text);
                let read_only = self.read_only;
                surface(geo, ui.context, |s| {
                    buffer.key_in(text, key, modifiers, read_only, s)
                })
            }
        }
    }
}
