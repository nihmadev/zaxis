//! NumberInput and DragValue for assistive technology: one spin button, whose requests go
//! through the steps of the arrow keys and the commit of a typed value.

use super::{
    render::{commit, step_from},
    NumberOptions, NumberState, Numeric,
};
use crate::{
    accessibility::Scope, AccessAction, AccessActionKind as Kind, AccessRole, Id, Rect, Ui,
};

/// A value set from outside is a commit: clamped, never snapped to the step, and the point
/// Escape returns to.
fn committed<T: Numeric>(state: &mut NumberState, value: T) {
    state.invalid = false;
    state.snapshot = value.to_string();
    state.origin = state.snapshot.clone();
    state.units = 0.0;
    state.buffer = state.snapshot.clone();
}

/// Apply the requests waiting for the spin button `node`.
pub(super) fn requests<T: Numeric>(
    ui: &mut Ui<'_>,
    node: Id,
    state: &mut NumberState,
    value: &mut T,
    options: &NumberOptions<'_, T>,
) {
    for action in ui.context.take_access_actions(node) {
        if !options.enabled {
            break;
        }
        match action {
            AccessAction::Increment | AccessAction::Decrement => {
                // A draft is committed first, as the arrow keys do: an incomplete one must
                // not turn into zero.
                if state.editing && !commit(&state.buffer, value, options) {
                    state.invalid = true;
                    continue;
                }
                let units = if action == AccessAction::Increment {
                    1.0
                } else {
                    -1.0
                };
                step_from(state, value, options, units);
                if state.editing {
                    state.buffer = value.to_string();
                }
            }
            AccessAction::SetNumericValue(next) if next.is_finite() => {
                let (min, max) = (*options.range.start(), *options.range.end());
                *value = T::from_f64(next).normalized(min, max);
                committed(state, *value);
            }
            AccessAction::SetValue(text) => {
                set_text(state, value, options, &text);
            }
            _ => {}
        }
    }
}

/// Commit `text` as if it had been typed and confirmed. The value is published with its
/// affixes, so it is accepted back with or without them; text that is no number is ignored.
pub(super) fn set_text<T: Numeric>(
    state: &mut NumberState,
    value: &mut T,
    options: &NumberOptions<'_, T>,
    text: &str,
) -> bool {
    let text = text.trim();
    let text = text.strip_prefix(options.prefix.as_str()).unwrap_or(text);
    let text = text.strip_suffix(options.suffix.as_str()).unwrap_or(text);
    let parsed = commit(text.trim(), value, options);
    if parsed {
        committed(state, *value);
    }
    parsed
}

/// Fill in the spin button opened as `scope` once this pass's value is final, and close it.
/// `hit` is the control's hit region and whether a text field was built on it: the field is
/// then the focused child, named after the spin button; otherwise the spin button holds
/// the focus itself.
pub(super) fn describe<T: Numeric>(
    ui: &mut Ui<'_>,
    scope: Scope,
    (hit, editor): (Id, bool),
    rect: Rect,
    value: T,
    options: &NumberOptions<'_, T>,
    state: &NumberState,
) {
    let Some(index) = scope.0.map(|index| index as usize) else {
        return;
    };
    let Some(node) = ui.context.a11y_node_mut(index) else {
        return;
    };
    let id = node.id;
    let shown = if editor {
        state.buffer.clone()
    } else {
        options.display(value)
    };
    node.value(format!("{}{shown}{}", options.prefix, options.suffix))
        .disabled(!options.enabled)
        .invalid(state.invalid && options.enabled)
        .action(Kind::Increment)
        .action(Kind::Decrement)
        .action(Kind::SetValue);
    // The whole range of the type is no range: only bounds somebody chose are announced.
    let (min, max) = (*options.range.start(), *options.range.end());
    if min != T::MIN || max != T::MAX {
        node.numeric(value.to_f64(), min.to_f64(), max.to_f64())
            .step(options.step.to_f64());
    }
    if editor {
        let field = ui.context.a11y_find(index + 1, |node| {
            matches!(
                node.role,
                AccessRole::TextInput | AccessRole::MultilineTextInput
            )
        });
        if let Some(field) = field.filter(|field| field.label.is_none()) {
            field.labelled_by(id);
        }
    } else {
        node.focus_on(hit);
    }
    ui.a11y_end(scope, Some(rect));
}
