//! The color editor for assistive technology: three sliders over the palette and the hue
//! strip, and the four channel fields. Requests take the paths of the keys and of a typed
//! commit.

use winit::keyboard::KeyCode;

use super::{editor::adjust, ColorPickerState, EditBuffer, FieldEdit};
use crate::{
    context::SliderInput, AccessAction, AccessActionKind as Kind, AccessOrientation, AccessRole,
    Color, Id, Rect, Ui, Vec2,
};

/// Name, full scale and unit of the hue, saturation and brightness channels.
const CHANNELS: [(&str, f32, &str); 3] = [
    ("Hue", 360.0, "\u{b0}"),
    ("Saturation", 100.0, "%"),
    ("Brightness", 100.0, "%"),
];
const FIELDS: [&str; 4] = ["Red", "Green", "Blue", "Hex"];

/// The node of the palette's brightness axis; its saturation axis is the palette itself.
fn brightness(palette: Id) -> Id {
    palette.with("brightness")
}

/// Apply the requests waiting for the sliders of one surface (the hue strip, or the palette
/// with saturation across and brightness up) and, while the editor is open, describe them.
/// Returns whether a request was applied.
pub(super) fn sliders(
    ui: &mut Ui<'_>,
    (control, rect, is_hue): (Id, Rect, bool),
    enabled: bool,
    open: bool,
    hsv: &mut [f32; 3],
) -> bool {
    let channels: &[usize] = if is_hue { &[0] } else { &[1, 2] };
    let mut applied = false;
    for &channel in channels {
        let up = channel == 2;
        let node = if up { brightness(control) } else { control };
        let (name, scale, unit) = CHANNELS[channel];
        for action in ui.context.take_access_actions(node) {
            if !enabled {
                break;
            }
            let event = match action {
                AccessAction::Increment if up => SliderInput::Key(KeyCode::ArrowUp),
                AccessAction::Decrement if up => SliderInput::Key(KeyCode::ArrowDown),
                AccessAction::Increment => SliderInput::Key(KeyCode::ArrowRight),
                AccessAction::Decrement => SliderInput::Key(KeyCode::ArrowLeft),
                // A value is a press at the point of the surface that stands for it.
                AccessAction::SetNumericValue(value) if value.is_finite() => {
                    let to = (value as f32 / scale).clamp(0.0, 1.0);
                    let mut at = if is_hue {
                        Vec2::new(hsv[0], 0.5)
                    } else {
                        Vec2::new(hsv[1], 1.0 - hsv[2])
                    };
                    if up {
                        at.y = 1.0 - to;
                    } else {
                        at.x = to;
                    }
                    SliderInput::Pointer(rect.min + rect.size() * at)
                }
                _ => continue,
            };
            adjust(hsv, rect, is_hue, event);
            applied = true;
        }
        if open {
            let value = (hsv[channel] * scale).round();
            ui.a11y(node, rect, AccessRole::Slider, |node| {
                node.label(name)
                    .value(format!("{value}{unit}"))
                    .numeric(f64::from(value), 0.0, f64::from(scale))
                    .step(f64::from(scale) / 100.0)
                    .jump(f64::from(scale) / 10.0)
                    .orientation(if up {
                        AccessOrientation::Vertical
                    } else {
                        AccessOrientation::Horizontal
                    })
                    .disabled(!enabled)
                    .action(Kind::Increment)
                    .action(Kind::Decrement)
                    .action(Kind::SetValue);
            });
        }
    }
    applied
}

/// A value set on a channel field is typed and committed: the same filter, the same limits.
pub(super) fn field_requests(
    ui: &mut Ui<'_>,
    fields: &[Id; 4],
    enabled: bool,
    state: &mut ColorPickerState,
    color: &mut Color,
) {
    for (field, id) in fields.iter().enumerate() {
        for action in ui.context.take_access_actions(*id) {
            let AccessAction::SetValue(text) = action else {
                continue;
            };
            if !enabled {
                break;
            }
            state.commit(color);
            let text = text
                .chars()
                .filter(|c| c.is_ascii() && !c.is_ascii_control())
                .take(16)
                .collect();
            state.edit = Some(FieldEdit {
                field,
                text,
                buffer: EditBuffer::default(),
            });
            state.commit(color);
        }
    }
}

/// One channel field, showing `text`.
pub(super) fn field(ui: &mut Ui<'_>, id: Id, rect: Rect, field: usize, text: &str, enabled: bool) {
    ui.a11y(id, rect, AccessRole::TextInput, |node| {
        node.label(FIELDS[field])
            .value(text)
            .disabled(!enabled)
            .action(Kind::SetValue);
    });
}
