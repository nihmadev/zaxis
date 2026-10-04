use super::*;
impl ColorPicker<'_> {
    pub(super) fn edit_field(&mut self, events: Vec<TextEditInput>, state: &mut ColorPickerState) {
        let field = state.edit.as_ref().unwrap().field;
        for event in events {
            if state.edit.is_none() {
                let text = field_text(*self.color, field);
                state.edit = Some(FieldEdit {
                    field,
                    buffer: EditBuffer {
                        cursor: text.len(),
                        anchor: 0,
                        ..Default::default()
                    },
                    text,
                });
            }
            let Some(edit) = state.edit.as_mut() else {
                break;
            };
            match event {
                TextEditInput::Text(text) => {
                    let text: String = text
                        .chars()
                        .filter(|c| c.is_ascii() && !c.is_ascii_control())
                        .collect();
                    if !text.is_empty() {
                        let remaining =
                            16usize.saturating_sub(edit.text.len() - edit.buffer.selection().len());
                        let text = &text[..text.len().min(remaining)];
                        edit.buffer.insert(&mut edit.text, text);
                    }
                }
                TextEditInput::Key(KeyCode::Enter, _) => state.commit(self.color),
                TextEditInput::Key(KeyCode::Escape, _) => state.edit = None,
                TextEditInput::Key(key, modifiers) => {
                    // Preserve RGB/HEX's non-extended selection and ASCII input rules.
                    let modifiers = if key == KeyCode::KeyA && modifiers.control_key() {
                        winit::keyboard::ModifiersState::CONTROL
                    } else {
                        winit::keyboard::ModifiersState::empty()
                    };
                    edit.buffer.key(&mut edit.text, key, modifiers, false);
                }
                _ => {}
            }
        }
    }
}
