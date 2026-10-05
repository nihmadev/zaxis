//! The color editor for assistive technology: three sliders over the palette and the hue
//! strip, and the four channel fields. Requests take the paths of the keys and of a typed
//! commit.

use winit::keyboard::KeyCode;

use super::{
    color::{field_text, HEX},
    editor::adjust,
    visible_label,
};
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

/// The row's color well, described before the editor runs; [`complete_well`] adds the
/// color once it is final. Returns where its node is.
pub(super) fn well(ui: &mut Ui<'_>, id: Id, row: Rect, text: &str, enabled: bool) -> usize {
    let well = ui.context.a11y_len();
    ui.a11y(id, row, AccessRole::ColorWell, |node| {
        node.label(visible_label(text))
            .disabled(!enabled)
            .clicks(id)
            .action(Kind::Expand)
            .action(Kind::Collapse);
    });
    well
}

/// The well's value is the final color of the pass, as hex.
pub(super) fn complete_well(ui: &mut Ui<'_>, well: usize, id: Id, color: Color, open: bool) {
    if let Some(node) = ui.context.a11y_node_mut(well).filter(|node| node.id == id) {
        node.value(field_text(color, HEX))
            .color(color)
            .expanded(open);
    }
}
