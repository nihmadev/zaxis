//! The red, green, blue and hex fields are real TextEdits: caret, selection, clipboard,
//! IME, undo and text geometry are theirs. A narrow adapter keeps each field's draft apart
//! from the color: Enter commits it, losing focus commits it, Escape drops it, and a value
//! from assistive technology takes the same commit. A draft that is no value never changes
//! the color. Fields take printable ASCII only, at most 16 characters.

use super::{
    color::{field_char, field_text, FIELDS, FIELD_LIMIT},
    ColorPickerState,
};
use crate::{
    components::theme::ControlStyle, context::TextEditInput, layout::LayoutCursor, Color,
    HoverStyle, Id, Layout, Rect, Response, TextEdit, Ui, Widget,
};
use winit::keyboard::KeyCode;

const NAMES: [&str; FIELDS] = ["Red", "Green", "Blue", "Hex"];

/// What the fields need from the picker in this pass.
pub(super) struct Fields<'s> {
    pub id: Id,
    pub enabled: bool,
    /// The editor is open; a closing editor keeps its fields out of the accessibility tree.
    pub open: bool,
    pub hover: Option<HoverStyle>,
    pub style: &'s crate::Style,
}

/// The id of field `field` of picker `id`.
pub(super) fn field_id(id: Id, field: usize) -> Id {
    id.with(("field", field))
}

/// Build the four fields in a row starting at `origin`, then commit the drafts of fields
/// that no longer have focus. Interaction of the fields is merged into `response`.
pub(super) fn show(
    ui: &mut Ui<'_>,
    fields: &Fields<'_>,
    (origin, width): (crate::Vec2, f32),
    (state, color): (&mut ColorPickerState, &mut Color),
    response: &mut Response,
) {
    let component = fields.style.color_picker;
    for field in 0..FIELDS {
        let rect = Rect::from_min_size(
            origin + crate::Vec2::new(width * field as f32 / FIELDS as f32, 0.0),
            crate::Vec2::new(
                (width / FIELDS as f32 - component.field_gap.unwrap_or(5.0)).max(0.0),
                fields.style.text_edit_height,
            ),
        );
        let field_response = edit(ui, fields, field, rect, state, color);
        response.has_focus |= field_response.has_focus;
        response.focus_visible |= field_response.focus_visible;
        response.pressed |= field_response.pressed;
    }
    let (context, id) = (&*ui.context, fields.id);
    if state.settle_drafts(color, |field| context.has_focus(field_id(id, field))) {
        // A committed field shows its value, normalized, on the next pass.
        ui.context.request_repaint();
    }
}

/// One field, placed at `rect` and styled like the picker's fields always were: the
/// theme's field fill and border, the picker's corner radius and field surface.
fn edit(
    ui: &mut Ui<'_>,
    fields: &Fields<'_>,
    field: usize,
    rect: Rect,
    state: &mut ColorPickerState,
    color: &mut Color,
) -> Response {
    let style = fields.style;
    let id = field_id(fields.id, field);
    let mut text = state.text(field, *color);
    let mut surface = ControlStyle::default();
    surface.idle.border = Some(style.border);
    surface.merge(style.color_picker.field);
    let response = {
        let mut adapter = Adapter {
            field,
            state: &mut *state,
            color: &mut *color,
        };
        let mut handler = |text: &mut String, event: &TextEditInput| adapter.handle(text, event);
        let mut editor = TextEdit::new(&mut text)
            .enabled(fields.enabled)
            .height(rect.size().y)
            .corner_radius(
                style
                    .color_picker
                    .rounding
                    .unwrap_or(style.text_edit_rounding),
            )
            .max_chars(FIELD_LIMIT)
            .style(crate::TextEditStyle {
                surface,
                ..Default::default()
            });
        if rect.size().x > 0.0 {
            editor = editor.width(rect.size().x);
        }
        if let Some(hover) = fields.hover {
            editor = editor.hover_style(hover);
        }
        editor.exact_id = Some(id);
        editor.whole_value = true;
        editor.accept = Some(field_char);
        editor.affixes = (["R ", "G ", "B ", ""][field].to_owned(), String::new());
        editor.event_handler = Some(&mut handler);
        let mut child = cell(ui, rect);
        if fields.open {
            editor.accessible_label(NAMES[field]).ui(&mut child)
        } else {
            editor.accessibility_hidden().ui(&mut child)
        }
    };
    state.update(field, text);
    response
}

/// A Ui laid out inside `rect`, clipped like its parent.
fn cell<'a>(ui: &'a mut Ui<'_>, rect: Rect) -> Ui<'a> {
    Ui {
        context: &mut *ui.context,
        window: ui.window,
        scope: ui.scope,
        sequence: 0,
        clip: ui.clip,
        layout: LayoutCursor::new(rect, Layout::Vertical, 0.0),
        enabled: ui.enabled,
        backdrop_blur: ui.backdrop_blur,
        hover_style: ui.hover_style,
        flow: None,
        local_style: ui.local_style.clone(),
        local_style_revision: ui.local_style_revision,
    }
}

/// Draft transitions of one field; editing itself is the TextEdit's.
struct Adapter<'a> {
    field: usize,
    state: &'a mut ColorPickerState,
    color: &'a mut Color,
}

impl Adapter<'_> {
    /// Handle one event; true when the TextEdit must not handle it too.
    fn handle(&mut self, text: &mut String, event: &TextEditInput) -> bool {
        let field = self.field;
        match event {
            // A field that lost focus is committed with the others at the end of the stage.
            TextEditInput::Focus(false) => false,
            TextEditInput::Key(KeyCode::Enter | KeyCode::NumpadEnter, _) => {
                self.state.submit(field, text.clone());
                true
            }
            TextEditInput::Key(KeyCode::Escape, _) => {
                self.state.cancel(field);
                *text = field_text(*self.color, field);
                true
            }
            TextEditInput::SetValue(value) => {
                // A value set from outside is typed and committed, after every draft.
                let value: String = value
                    .chars()
                    .filter(|c| field_char(*c))
                    .take(FIELD_LIMIT)
                    .collect();
                self.state.submit_all();
                self.state.submit(field, value.clone());
                *text = value;
                true
            }
            _ => {
                self.state.begin(field, text);
                false
            }
        }
    }
}
