use std::{hash::Hash, panic::Location};

use winit::keyboard::KeyCode;

use crate::{
    context::{HitAction, HitRegion, Paint, SliderInput, TextEditInput},
    Border, Color, CornerRadius, Id, Padding, Rect, Shape, Vec2,
};

use super::{edit_buffer::EditBuffer, visible_label, HoverStyle, Response, Ui, Widget, Window};

/// Where the expanded color editor is displayed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorPickerType {
    /// Expand below the color row and allocate space in its layout.
    #[default]
    Internal,
    /// Open an independent draggable window without expanding the parent layout.
    Floating,
}

/// A Rayfield-style color row with an HSV palette, hue strip, and RGB/HEX fields.
/// Changes preserve the bound color's alpha channel.
pub struct ColorPicker<'a> {
    color: &'a mut Color,
    text: String,
    id: Option<Id>,
    width: Option<f32>,
    enabled: bool,
    picker_type: ColorPickerType,
    default_open: bool,
    source: &'static Location<'static>,
    hover_style: Option<HoverStyle>,
    style: crate::ColorPickerStyle,
}

impl<'a> ColorPicker<'a> {
    #[track_caller]
    pub fn new(color: &'a mut Color, text: impl Into<String>) -> Self {
        Self {
            color,
            text: text.into(),
            id: None,
            width: None,
            enabled: true,
            picker_type: ColorPickerType::Internal,
            default_open: false,
            source: Location::caller(),
            hover_style: None,
            style: Default::default(),
        }
    }

    pub fn style(mut self, style: crate::ColorPickerStyle) -> Self {
        self.style = style;
        self
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        assert!(
            width.is_finite() && width > 0.0,
            "color picker width must be finite and positive"
        );
        self.width = Some(width);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }

    pub fn picker_type(mut self, picker_type: ColorPickerType) -> Self {
        self.picker_type = picker_type;
        self
    }

    /// Initial expansion state. Subsequent interaction is retained by widget ID.
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    pub fn hover_style(mut self, style: HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
    }
}

impl ColorPicker<'_> {
    fn editor_height(&self, style: &crate::Style) -> f32 {
        let c = style.color_picker;
        c.palette_height.unwrap_or(112.0)
            + c.hue_height.unwrap_or(14.0)
            + c.gap.unwrap_or(10.0) * 3.0
            + 4.0
            + style.text_edit_height
            + 6.0
    }
}
pub(crate) struct ColorPickerState {
    open: bool,
    hsv: [f32; 3],
    last_color: Color,
    edit: Option<FieldEdit>,
}

struct FieldEdit {
    field: usize,
    text: String,
    buffer: EditBuffer,
}

impl ColorPickerState {
    fn sync(&mut self, color: Color) {
        if self.last_color != color {
            let hsv = to_hsv(color);
            // Retain the selected hue for greys, and saturation when choosing black.
            if hsv[1] > 0.0 {
                self.hsv[0] = hsv[0];
            }
            if hsv[2] > 0.0 {
                self.hsv[1] = hsv[1];
            }
            self.hsv[2] = hsv[2];
            self.last_color = color;
        }
    }

    fn commit(&mut self, color: &mut Color) {
        if let Some(edit) = self.edit.take() {
            if edit.field == 3 {
                let text = edit.text.trim().trim_start_matches('#');
                if text.len() == 6 && text.bytes().all(|b| b.is_ascii_hexdigit()) {
                    let rgb = u32::from_str_radix(text, 16).unwrap();
                    *color =
                        Color::rgba((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, color.0[3]);
                }
            } else if let Ok(value) = edit.text.trim().parse::<i64>() {
                color.0[edit.field] = value.clamp(0, 255) as u8;
            }
            self.sync(*color);
        }
    }
}

fn field_text(color: Color, field: usize) -> String {
    if field == 3 {
        format!("#{:02X}{:02X}{:02X}", color.0[0], color.0[1], color.0[2])
    } else {
        color.0[field].to_string()
    }
}

fn to_hsv(color: Color) -> [f32; 3] {
    let [r, g, b] = [color.0[0], color.0[1], color.0[2]].map(|c| f32::from(c) / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        ((g - b) / delta).rem_euclid(6.0) / 6.0
    } else if max == g {
        ((b - r) / delta + 2.0) / 6.0
    } else {
        ((r - g) / delta + 4.0) / 6.0
    };
    [hue, if max == 0.0 { 0.0 } else { delta / max }, max]
}

fn from_hsv([h, s, v]: [f32; 3], alpha: u8) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let rgb = match h as u32 {
        0 => [c, x, 0.0],
        1 => [x, c, 0.0],
        2 => [0.0, c, x],
        3 => [0.0, x, c],
        4 => [x, 0.0, c],
        _ => [c, 0.0, x],
    }
    .map(|channel| ((channel + v - c) * 255.0).round() as u8);
    Color::rgba(rgb[0], rgb[1], rgb[2], alpha)
}

fn rounded(rect: Rect, fill: Color, radius: impl Into<CornerRadius>, border: Border) -> Paint {
    Paint::Shape(
        Shape::rect(rect, fill)
            .corner_radius(radius)
            .border(border)
            .into(),
    )
}

fn hit(ui: &mut Ui<'_>, id: Id, rect: Rect, enabled: bool, action: HitAction) {
    ui.context.register_hit(HitRegion {
        id,
        window: ui.window,
        rect,
        clip: ui.clip,
        action: if enabled { action } else { HitAction::Block },
    });
}

impl Ui<'_> {
    #[track_caller]
    pub fn color_picker(&mut self, color: &mut Color, text: impl Into<String>) -> Response {
        self.add(ColorPicker::new(color, text))
    }
}

mod editor;
mod fields;
mod show;
#[cfg(test)]
mod tests;
