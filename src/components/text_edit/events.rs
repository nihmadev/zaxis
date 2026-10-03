use super::*;

impl TextEdit<'_> {
    pub(super) fn process_events(
        &mut self,
        ui: &mut Ui<'_>,
        state: &mut TextEditState,
        response: &mut Response,
        events: Vec<TextEditInput>,
        inner: Rect,
        size: f32,
    ) {
        for event in events {
            if self.enabled
                && (state.preedit.is_none()
                    || matches!(
                        event,
                        TextEditInput::Focus(_) | TextEditInput::Key(KeyCode::Escape, _)
                    ))
            {
                if let Some(handler) = &mut self.event_handler {
                    let previous = self.text.clone();
                    let handled = handler(self.text, &event);
                    if previous != *self.text {
                        state.buffer.select_all(self.text);
                        state.history = EditHistory::default();
                    }
                    if handled {
                        if matches!(event, TextEditInput::Key(KeyCode::Escape, _)) {
                            state.preedit = None;
                        }
                        continue;
                    }
                }
            }
            let previous = self.text.clone();
            let previous_buffer = state.buffer.clone();
            let typing = matches!(&event, TextEditInput::Text(text)
                if state.preedit.is_none() && previous_buffer.selection().is_empty()
                    && text.graphemes(true).count() == 1 && !text.chars().any(char::is_control));
            let restoring = matches!(&event, TextEditInput::Key(key, mods)
                if (mods.control_key() || mods.super_key())
                    && matches!(key, KeyCode::KeyZ | KeyCode::KeyY));
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
                                | KeyCode::Backspace
                                | KeyCode::Delete
                                | KeyCode::Enter
                                | KeyCode::NumpadEnter
                                | KeyCode::Escape
                        )
                }
                _ => !typing,
            };
            if interrupts_typing {
                state.history.break_group();
            }
            match event {
                TextEditInput::Focus(false) => {
                    response.lost_focus = true;
                    state.preedit = None;
                }
                TextEditInput::Focus(true) => {}
                _ if !self.enabled => {}
                TextEditInput::Text(text) | TextEditInput::Commit(text) if !self.read_only => {
                    state.preedit = None;
                    let text = single_line(&text);
                    if !text.is_empty() {
                        state.buffer.insert(self.text, &text);
                    }
                }
                TextEditInput::Preedit(text, cursor) if !self.read_only => {
                    state.preedit = (!text.is_empty()).then_some((single_line(&text), cursor));
                }
                TextEditInput::Pointer(pointer, extend, count) => {
                    state.preedit = None;
                    let points = positions(ui, self.text, size);
                    let x = pointer.x - inner.min.x + state.scroll;
                    let cursor = points
                        .iter()
                        .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
                        .map_or(0, |p| p.0);
                    let word_cursor = points
                        .iter()
                        .rev()
                        .find(|(_, px)| *px <= x)
                        .map_or(0, |p| p.0);
                    match count {
                        2 => {
                            let word = word_at(self.text, word_cursor);
                            state.buffer.anchor = word.start;
                            state.buffer.cursor = word.end;
                            state.word_drag = Some(word);
                        }
                        3 => {
                            state.buffer.select_all(self.text);
                            state.word_drag = Some(0..self.text.len());
                        }
                        0 if state.word_drag.is_some() => {
                            let anchor = state.word_drag.as_ref().unwrap();
                            let word = word_at(self.text, word_cursor);
                            if word.start < anchor.start {
                                state.buffer.anchor = anchor.end;
                                state.buffer.cursor = word.start;
                            } else {
                                state.buffer.anchor = anchor.start;
                                state.buffer.cursor = word.end.max(anchor.end);
                            }
                        }
                        _ => {
                            state.word_drag = None;
                            state.buffer.set_cursor(cursor, extend);
                        }
                    }
                }
                TextEditInput::Key(key, modifiers) if state.preedit.is_none() => {
                    let command = modifiers.control_key() || modifiers.super_key();
                    match key {
                        KeyCode::KeyZ | KeyCode::KeyY if command && !self.read_only => {
                            let redo = key == KeyCode::KeyY || modifiers.shift_key();
                            state.history.restore(self.text, &mut state.buffer, redo);
                            state.word_drag = None;
                        }
                        KeyCode::Enter | KeyCode::NumpadEnter => response.submitted = true,
                        KeyCode::KeyC | KeyCode::KeyX if command => {
                            let range = state.buffer.selection();
                            if !range.is_empty()
                                && ui.context.copy_text(self.text[range].to_owned()).is_ok()
                                && key == KeyCode::KeyX
                                && !self.read_only
                            {
                                state.buffer.insert(self.text, "");
                            }
                        }
                        KeyCode::KeyV if command && !self.read_only => {
                            if let Ok(text) = ui.context.clipboard_text() {
                                let text = single_line(&text);
                                if !text.is_empty() {
                                    state.buffer.insert(self.text, &text);
                                }
                            }
                        }
                        _ => state.buffer.key(self.text, key, modifiers, self.read_only),
                    }
                }
                _ => {}
            }
            if previous != *self.text && !restoring {
                state.history.record(previous, previous_buffer, typing);
                state.word_drag = None;
            }
        }
    }
}
