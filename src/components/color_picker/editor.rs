//! The expanded editor: a saturation/brightness palette and a hue strip above the channel
//! fields. The surfaces register their regions and take their input first (which keeps the
//! Tab order palette, hue, fields), then the fields run, then the surfaces are painted
//! with the color both left behind.

use super::{access, fields, hit, rounded, ColorPickerState, HitAction, Paint, Shape, SliderInput};
use crate::{Border, Color, CornerRadius, HoverStyle, Id, Rect, Response, Ui, Vec2};
use winit::keyboard::KeyCode;

/// What the editor of one picker works with in this pass.
#[derive(Clone, Copy)]
pub(super) struct Editor<'s> {
    pub id: Id,
    pub enabled: bool,
    pub hover: Option<HoverStyle>,
    /// The resolved style, the picker's own overrides merged in.
    pub style: &'s crate::Style,
}

/// Where the parts of the editor go.
struct Geometry {
    palette: Rect,
    hue: Rect,
    fields: Vec2,
}

impl Geometry {
    fn new(origin: Vec2, width: f32, style: &crate::Style) -> Self {
        let component = style.color_picker;
        let palette_height = component.palette_height.unwrap_or(112.0);
        let hue_height = component.hue_height.unwrap_or(14.0);
        let gap = component.gap.unwrap_or(10.0);
        Self {
            palette: Rect::from_min_size(origin, Vec2::new(width, palette_height)),
            hue: Rect::from_min_size(
                origin + Vec2::new(0.0, palette_height + gap),
                Vec2::new(width, hue_height),
            ),
            fields: origin + Vec2::new(0.0, palette_height + hue_height + gap * 2.0 + 4.0),
        }
    }
}

pub(super) fn show(
    ui: &mut Ui<'_>,
    editor: &Editor<'_>,
    (origin, width): (Vec2, f32),
    (state, color): (&mut ColorPickerState, &mut Color),
    response: &mut Response,
) {
    let geometry = Geometry::new(origin, width, editor.style);
    for (control, rect, is_hue) in [
        (editor.id.with("palette"), geometry.palette, false),
        (editor.id.with("hue"), geometry.hue, true),
    ] {
        surface_input(
            ui,
            editor,
            (control, rect, is_hue),
            (state, color),
            response,
        );
    }
    let fields = fields::Fields {
        id: editor.id,
        enabled: editor.enabled,
        open: state.open,
        hover: editor.hover,
        style: editor.style,
    };
    fields::show(
        ui,
        &fields,
        (geometry.fields, width),
        (state, color),
        response,
    );
    paint_surfaces(ui, editor, &geometry, state.hsv);
}

/// Register a surface and apply its pointer, key and assistive-technology input.
fn surface_input(
    ui: &mut Ui<'_>,
    editor: &Editor<'_>,
    surface: (Id, Rect, bool),
    (state, color): (&mut ColorPickerState, &mut Color),
    response: &mut Response,
) {
    let (control_id, rect, is_hue) = surface;
    let control = ui.response(control_id, rect, editor.enabled);
    response.pressed |= control.pressed;
    response.has_focus |= control.has_focus;
    response.focus_visible |= control.focus_visible;
    hit(ui, control_id, rect, editor.enabled, HitAction::Slider);
    for event in ui.context.take_slider_input(control_id) {
        if !editor.enabled {
            continue;
        }
        adjust(&mut state.hsv, rect, is_hue, event);
        state.set_hsv(color);
    }
    if access::sliders(ui, surface, editor.enabled, state.open, &mut state.hsv) {
        state.set_hsv(color);
    }
}

/// One pointer or key event on the palette (saturation across, brightness up) or on the hue
/// strip.
pub(super) fn adjust(hsv: &mut [f32; 3], rect: Rect, is_hue: bool, event: SliderInput) {
    match event {
        SliderInput::Pointer(pointer) if !rect.is_empty() => {
            let t = ((pointer - rect.min) / rect.size()).clamp(Vec2::ZERO, Vec2::ONE);
            if is_hue {
                hsv[0] = t.x;
            } else {
                hsv[1] = t.x;
                hsv[2] = 1.0 - t.y;
            }
        }
        SliderInput::Key(key) => {
            let channel = if is_hue {
                0
            } else if matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown) {
                2
            } else {
                1
            };
            hsv[channel] = match key {
                KeyCode::Home => 0.0,
                KeyCode::End => 1.0,
                KeyCode::ArrowRight | KeyCode::ArrowUp => (hsv[channel] + 0.01).min(1.0),
                KeyCode::ArrowLeft | KeyCode::ArrowDown => (hsv[channel] - 0.01).max(0.0),
                KeyCode::PageUp => (hsv[channel] + 0.1).min(1.0),
                KeyCode::PageDown => (hsv[channel] - 0.1).max(0.0),
                _ => hsv[channel],
            };
        }
        _ => {}
    }
}

/// The palette at the current hue, the hue strip, and the markers and focus rings.
fn paint_surfaces(ui: &mut Ui<'_>, editor: &Editor<'_>, geometry: &Geometry, hsv: [f32; 3]) {
    let (id, style) = (editor.id, editor.style);
    let rounding = style
        .color_picker
        .rounding
        .unwrap_or(CornerRadius::all(4.0));
    let (palette, hue) = (geometry.palette, geometry.hue);
    let mut colors = Vec::with_capacity(33 * 17);
    for row in 0..17 {
        for column in 0..33 {
            colors.push(super::from_hsv(
                [hsv[0], column as f32 / 32.0, 1.0 - row as f32 / 16.0],
                255,
            ));
        }
    }
    let gradient = |rect, colors, columns, rows| {
        vec![Paint::Gradient {
            rect,
            rounding,
            colors,
            columns,
            rows,
        }]
    };
    ui.context.paint(
        id.with("palette-gradient"),
        ui.window,
        ui.clip,
        gradient(palette, colors, 33, 17),
    );
    let strip = (0..2)
        .flat_map(|_| (0..49).map(|column| super::from_hsv([column as f32 / 48.0, 1.0, 1.0], 255)))
        .collect();
    ui.context.paint(
        id.with("hue-gradient"),
        ui.window,
        ui.clip,
        gradient(hue, strip, 49, 2),
    );
    let point = palette.min + palette.size() * Vec2::new(hsv[1], 1.0 - hsv[2]);
    let thumb = Rect::from_min_size(
        Vec2::new(hue.min.x + palette.size().x * hsv[0] - 2.0, hue.min.y - 2.0),
        Vec2::new(4.0, hue.size().y + 4.0),
    );
    let ring = |control: &str| {
        if ui.context.focus_visible(id.with(control)) {
            style.focus_border
        } else {
            Border::NONE
        }
    };
    let markers = vec![
        Paint::Shape(Shape::Circle {
            center: point,
            radius: 5.0,
            fill: Color::TRANSPARENT,
            border: Border::new(3.0, Color::BLACK),
        }),
        Paint::Shape(Shape::Circle {
            center: point,
            radius: 4.0,
            fill: Color::TRANSPARENT,
            border: Border::new(2.0, Color::WHITE),
        }),
        rounded(thumb, Color::WHITE, 2.0, Border::new(1.0, Color::gray(40))),
        rounded(palette, Color::TRANSPARENT, rounding, ring("palette")),
        rounded(hue, Color::TRANSPARENT, rounding, ring("hue")),
    ];
    ui.context
        .paint(id.with("markers"), ui.window, ui.clip, markers);
}
