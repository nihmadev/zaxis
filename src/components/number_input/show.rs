//! One pass of a numeric control: identity and style, the retained state brought up to date
//! with the bound value, requests of assistive technology, drag input, the field or the
//! drag surface, and the response. NumberInput and DragValue share every stage; a drag
//! value is a field only while it is edited.

use super::{access, display, drag, editor, NumberOptions, NumberStyle, Numeric};
use crate::{Border, Color, Id, Response, Shape, Ui};

pub(super) fn show<T: Numeric>(
    ui: &mut Ui<'_>,
    value: &mut T,
    mut options: NumberOptions<'_, T>,
    drag: bool,
) -> Response {
    options.enabled &= ui.is_enabled();
    let id = control_id(ui, &options, drag);
    let style = resolve_style(ui, &options);
    let before = *value;
    let mut state = ui.context.values.numbers.remove(&id).unwrap_or_default();
    let was_editing = state.editing;
    state.last_frame = ui.context.frame;
    let focused = ui.context.has_focus(id);
    state.observe(*value, options.enabled);
    if !drag && focused && !state.editing && options.enabled {
        state.begin(*value);
    }
    let node = id.with("spin");
    access::requests(ui, node, &mut state, value, &options);
    let events = ui.context.take_number_input(id);
    if drag && options.enabled {
        drag::apply(&mut state, value, &options, &style, events);
    }
    // One spin button either way; the text field of an edit is its child.
    let field = !drag || state.editing;
    let scope = ui.a11y_begin(node, crate::AccessRole::SpinButton, |_| {});
    let (mut response, outcome) = if field {
        let began = state.editing && !was_editing;
        editor::show(ui, id, (&mut state, value), &options, &style, (drag, began))
    } else {
        let response = display::show(ui, id, *value, &options, &style);
        (response, editor::Outcome::default())
    };
    if state.invalid && options.enabled {
        paint_invalid(ui, id, &response, &style);
    }
    access::describe(
        ui,
        scope,
        (id, field),
        response.rect,
        *value,
        &options,
        &state,
    );
    response.changed = !before.same(*value);
    response.submitted = outcome.submitted;
    response.lost_focus |= state.focused && !focused;
    state.focused = focused;
    state.last_value = value.to_string();
    if outcome.submitted || outcome.cancelled {
        ui.context.request_repaint();
    }
    ui.context.values.numbers.insert(id, state);
    response
}

fn control_id<T: Numeric>(ui: &mut Ui<'_>, options: &NumberOptions<'_, T>, drag: bool) -> Id {
    match options.id {
        Some(id) => ui.scope.with(("number", id)),
        None => ui.auto_id((
            if drag { "drag-value" } else { "number-input" },
            options.source,
        )),
    }
}

/// The theme's numeric style with the control's overrides, normalized.
fn resolve_style<T: Numeric>(ui: &Ui<'_>, options: &NumberOptions<'_, T>) -> NumberStyle {
    let mut style = options
        .style
        .clone()
        .unwrap_or_else(|| ui.style().number.clone());
    style.width = options.width.unwrap_or(style.width);
    style.sensitivity = options.sensitivity.unwrap_or(style.sensitivity);
    style.rounding = options.rounding.unwrap_or(style.rounding);
    style.normalize();
    style
}

/// A draft that is no number is outlined, inside the focus ring when focused.
fn paint_invalid(ui: &mut Ui<'_>, id: Id, response: &Response, style: &NumberStyle) {
    let inset = if response.has_focus { 2.0 } else { 0.0 };
    ui.context.paint(
        id.with("invalid"),
        ui.window,
        ui.clip,
        vec![crate::context::Paint::Shape(
            Shape::rect(response.rect.shrink(inset), Color::TRANSPARENT)
                .corner_radius(style.rounding)
                .border(Border::new(1.0, style.invalid))
                .into(),
        )],
    );
}
