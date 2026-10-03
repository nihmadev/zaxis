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
    width: f32,
    enabled: bool,
    picker_type: ColorPickerType,
    default_open: bool,
    source: &'static Location<'static>,
    hover_style: Option<HoverStyle>,
}

impl<'a> ColorPicker<'a> {
    #[track_caller]
    pub fn new(color: &'a mut Color, text: impl Into<String>) -> Self {
        Self {
            color,
            text: text.into(),
            id: None,
            width: 300.0,
            enabled: true,
            picker_type: ColorPickerType::Internal,
            default_open: false,
            source: Location::caller(),
            hover_style: None,
        }
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
        self.width = width;
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

fn rounded(rect: Rect, fill: Color, radius: f32, border: Border) -> Paint {
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

impl Widget for ColorPicker<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let id = match self.id {
            Some(id) => ui.scope.with(("color-picker", id)),
            None => ui.auto_id(("color-picker", self.source)),
        };
        let original = *self.color;
        let mut state = ui
            .context
            .color_pickers
            .remove(&id)
            .unwrap_or_else(|| ColorPickerState {
                open: self.default_open,
                hsv: to_hsv(original),
                last_color: original,
                edit: None,
            });
        if original != state.last_color {
            state.edit = None;
        }
        state.sync(original);
        let width = self.width.min(ui.available_width());
        let row = Rect::from_min_size(ui.layout.cursor, Vec2::new(width, 24.0));
        let mut response = ui.response(id, row, self.enabled);
        if response.clicked() {
            state.open = !state.open;
            if !state.open {
                state.commit(self.color);
            }
        }
        if !self.enabled {
            state.edit = None;
        }
        let open = state.open;
        let style = ui.style().clone();
        let angle = ui.transition_property(
            id.with("chevron-angle"),
            row,
            if open { std::f32::consts::PI } else { 0.0 },
            style.motion.expand.clone(),
        );
        let mut rect = ui.allocate_space(Vec2::new(width, 24.0));
        response.rect = rect;
        hit(ui, id, row, self.enabled, HitAction::Activate);
        let swatch = Rect::from_min_size(
            Vec2::new((row.max.x - 28.0).max(row.min.x), row.min.y + 4.0),
            Vec2::new(width.min(28.0), 16.0),
        );
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let mut row_response = response;
        row_response.rect = row;
        let row_hover = ui.animate_hover(
            row_response,
            preset,
            super::appearance::Appearance::new(
                Color::TRANSPARENT,
                Border::NONE,
                if self.enabled {
                    style.text_color
                } else {
                    style.muted_text
                },
            ),
            style.button_hovered,
        );
        let mut row_paint = Vec::new();
        row_hover.paint_shadow(row, CornerRadius::all(4.0), &mut row_paint);
        row_hover.paint_body(row, CornerRadius::all(4.0), &style, 0.0, &mut row_paint);
        ui.context
            .paint(id.with("row"), ui.window, ui.clip, row_paint);

        if self.picker_type == ColorPickerType::Internal || open {
            match self.picker_type {
                ColorPickerType::Internal => {
                    let enabled = self.enabled;
                    self.enabled &= open;
                    let reveal_id = ui.scope.with(("reveal", Id::new(id)));
                    if open && !ui.context.effect_states.contains_key(&reveal_id) {
                        // Preserve the existing first-appearance snap for default_open.
                        ui.context
                            .transition(reveal_id, 196.0_f32, style.motion.expand.clone());
                    }
                    ui.layout.cursor.y = row.max.y;
                    ui.reveal(id, open, |ui| {
                        let origin = ui.layout.cursor + Vec2::new(0.0, 10.0);
                        self.editor(ui, id, origin, width, &mut state, &mut response);
                        ui.allocate_space(Vec2::new(width, 196.0));
                    });
                    let height = ui
                        .context
                        .effect_states
                        .get(&reveal_id)
                        .map_or(0.0, |s| s.value);
                    rect.max.y = row.max.y + height;
                    response.rect = rect;
                    self.enabled = enabled;
                    if ui.context.input().keys_pressed.contains(&KeyCode::Escape)
                        && response.has_focus
                        && !(0usize..4).any(|field| ui.context.has_focus(id.with(("field", field))))
                    {
                        state.open = false;
                        ui.context.request_repaint();
                    }
                }
                ColorPickerType::Floating if self.enabled => {
                    let floating_id = id.with("floating");
                    let size = Vec2::new(
                        self.width.max(260.0) + 24.0,
                        style.title_height.max(24.0) + 200.0,
                    );
                    let viewport = ui.context.viewport().size();
                    let position = Vec2::new(row.min.x, row.max.y + 8.0)
                        .min((viewport - size).max(Vec2::ZERO))
                        .max(Vec2::ZERO);
                    Window::new(visible_label(&self.text))
                        .id(floating_id)
                        .default_position(position)
                        .default_size(size)
                        .min_size(size)
                        .resizable(false)
                        .padding(Padding::all(12.0))
                        .show(ui.context, |popup| {
                            let origin = popup.layout.cursor;
                            let width = popup.available_width();
                            self.editor(popup, id, origin, width, &mut state, &mut response);
                            let window = popup.context.windows[&floating_id].rect;
                            let close = Rect::from_min_size(
                                Vec2::new(window.max.x - 30.0, window.min.y + 6.0),
                                Vec2::splat(24.0),
                            );
                            let close_id = id.with("close");
                            let close_response = popup.response(close_id, close, true);
                            popup.context.register_hit(HitRegion {
                                id: close_id,
                                window: floating_id,
                                rect: close,
                                clip: window.intersect(popup.context.viewport()),
                                action: HitAction::Activate,
                            });
                            let preset = self
                                .hover_style
                                .or(popup.hover_style)
                                .unwrap_or(style.hover_style);
                            let hover = popup.animate_hover(
                                close_response,
                                preset,
                                super::appearance::Appearance::new(
                                    Color::TRANSPARENT,
                                    if close_response.focus_visible {
                                        style.focus_border
                                    } else {
                                        Border::NONE
                                    },
                                    style.text_color,
                                ),
                                style.button_hovered,
                            );
                            let mut paint = Vec::new();
                            hover.paint_shadow(close, CornerRadius::all(4.0), &mut paint);
                            hover.paint_body(
                                close,
                                CornerRadius::all(4.0),
                                &style,
                                0.0,
                                &mut paint,
                            );
                            for direction in [-1.0, 1.0] {
                                paint.push(Paint::Shape(Shape::Line {
                                    start: close.center() + Vec2::new(-4.0, -4.0 * direction),
                                    end: close.center() + Vec2::new(4.0, 4.0 * direction),
                                    width: 1.5,
                                    color: style.text_color,
                                }));
                            }
                            popup.context.paint(
                                close_id.with("body"),
                                floating_id,
                                window.intersect(popup.context.viewport()),
                                paint,
                            );
                            // Escape in a field first cancels that field's edit.
                            let field_focused = (0usize..4)
                                .any(|field| popup.context.has_focus(id.with(("field", field))));
                            if close_response.clicked()
                                || (popup
                                    .context
                                    .input()
                                    .keys_pressed
                                    .contains(&KeyCode::Escape)
                                    && !field_focused
                                    && popup.context.front_window() == Some(floating_id))
                            {
                                state.commit(self.color);
                                state.open = false;
                                popup.context.request_repaint();
                            }
                        });
                    if response.clicked() {
                        ui.context.raise_window(floating_id);
                    }
                }
                ColorPickerType::Floating => {}
            }
        }
        // Paint after editing so the preview reflects this pass's final color.
        let mut swatch_paint = Vec::new();
        // Preserve the actual selected color when the row uses a hover gradient.
        swatch_paint.push(rounded(swatch, *self.color, 4.0, style.border));
        ui.context
            .paint(id.with("swatch"), ui.window, ui.clip, swatch_paint);
        ui.context.paint(
            id.with("caption"),
            ui.window,
            ui.clip.intersect(Rect::from_min_max(
                row.min,
                Vec2::new(swatch.min.x - 26.0, row.max.y),
            )),
            vec![Paint::Text {
                text: visible_label(&self.text).to_owned(),
                position: row.min + Vec2::new(0.0, 2.0),
                size: super::font_size(style.font_size),
                wrap_width: f32::INFINITY,
                color: row_hover.text_color,
            }],
        );
        let center = Vec2::new(swatch.min.x - 14.0, row.center().y);
        let rotate = |v: Vec2| {
            Vec2::new(
                v.x * angle.cos() - v.y * angle.sin(),
                v.x * angle.sin() + v.y * angle.cos(),
            ) + center
        };
        ui.context.paint(
            id.with("chevron"),
            ui.window,
            ui.clip,
            vec![
                Paint::Shape(Shape::Line {
                    start: rotate(Vec2::new(-4.0, -2.0)),
                    end: rotate(Vec2::new(0.0, 2.0)),
                    width: 1.5,
                    color: row_hover.text_color,
                }),
                Paint::Shape(Shape::Line {
                    start: rotate(Vec2::new(0.0, 2.0)),
                    end: rotate(Vec2::new(4.0, -2.0)),
                    width: 1.5,
                    color: row_hover.text_color,
                }),
            ],
        );
        ui.context.paint(
            id.with("focus"),
            ui.window,
            ui.clip,
            vec![rounded(
                row,
                Color::TRANSPARENT,
                4.0,
                if ui.context.focus_visible(id) {
                    style.focus_border
                } else {
                    Border::NONE
                },
            )],
        );
        response.changed = original != *self.color;
        state.last_color = *self.color;
        ui.context.color_pickers.insert(id, state);
        response
    }
}

impl ColorPicker<'_> {
    fn editor(
        &mut self,
        ui: &mut Ui<'_>,
        id: Id,
        origin: Vec2,
        width: f32,
        state: &mut ColorPickerState,
        response: &mut Response,
    ) {
        let style = ui.style().clone();
        let palette = Rect::from_min_size(origin, Vec2::new(width, 112.0));
        let hue = Rect::from_min_size(origin + Vec2::new(0.0, 122.0), Vec2::new(width, 14.0));
        let field_ids = std::array::from_fn::<_, 4, _>(|field| id.with(("field", field)));
        let focused = field_ids
            .iter()
            .position(|field| self.enabled && ui.context.has_focus(*field));
        // Include fields that received input and lost focus between two UI passes.
        let previous = state.edit.as_ref().map(|edit| edit.field);
        let fields = previous
            .into_iter()
            .chain((0..4).filter(|field| Some(*field) != previous));
        for field in fields {
            let events = ui.context.take_text_edit_input(field_ids[field]);
            if self.enabled && !events.is_empty() {
                if state.edit.as_ref().is_none_or(|edit| edit.field != field) {
                    state.commit(self.color);
                    let text = field_text(*self.color, field);
                    state.edit = Some(FieldEdit {
                        field,
                        buffer: EditBuffer {
                            cursor: text.len(),
                            anchor: 0,
                        },
                        text,
                    });
                }
                self.edit_field(events, state);
            }
        }
        if state
            .edit
            .as_ref()
            .is_some_and(|edit| focused != Some(edit.field))
        {
            state.commit(self.color);
        }
        if let Some(field) = focused {
            if state.edit.is_none() {
                let text = field_text(*self.color, field);
                state.edit = Some(FieldEdit {
                    field,
                    buffer: EditBuffer {
                        cursor: text.len(),
                        anchor: 0,
                    },
                    text,
                });
            }
        }
        for (control_id, rect, is_hue) in [
            (id.with("palette"), palette, false),
            (id.with("hue"), hue, true),
        ] {
            let control = ui.response(control_id, rect, self.enabled);
            response.pressed |= control.pressed;
            response.has_focus |= control.has_focus;
            response.focus_visible |= control.focus_visible;
            hit(ui, control_id, rect, self.enabled, HitAction::Slider);
            for event in ui.context.take_slider_input(control_id) {
                if !self.enabled {
                    continue;
                }
                match event {
                    SliderInput::Pointer(pointer) if !rect.is_empty() => {
                        let t = ((pointer - rect.min) / rect.size()).clamp(Vec2::ZERO, Vec2::ONE);
                        if is_hue {
                            state.hsv[0] = t.x;
                        } else {
                            state.hsv[1] = t.x;
                            state.hsv[2] = 1.0 - t.y;
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
                        state.hsv[channel] = match key {
                            KeyCode::Home => 0.0,
                            KeyCode::End => 1.0,
                            KeyCode::ArrowRight | KeyCode::ArrowUp => {
                                (state.hsv[channel] + 0.01).min(1.0)
                            }
                            KeyCode::ArrowLeft | KeyCode::ArrowDown => {
                                (state.hsv[channel] - 0.01).max(0.0)
                            }
                            KeyCode::PageUp => (state.hsv[channel] + 0.1).min(1.0),
                            KeyCode::PageDown => (state.hsv[channel] - 0.1).max(0.0),
                            _ => state.hsv[channel],
                        };
                    }
                    _ => {}
                }
                *self.color = from_hsv(state.hsv, self.color.0[3]);
            }
        }
        let mut colors = Vec::with_capacity(33 * 17);
        for row in 0..17 {
            for column in 0..33 {
                colors.push(from_hsv(
                    [state.hsv[0], column as f32 / 32.0, 1.0 - row as f32 / 16.0],
                    255,
                ));
            }
        }
        ui.context.paint(
            id.with("palette-gradient"),
            ui.window,
            ui.clip,
            vec![Paint::Gradient {
                rect: palette,
                rounding: CornerRadius::all(4.0),
                colors,
                columns: 33,
                rows: 17,
            }],
        );
        ui.context.paint(
            id.with("hue-gradient"),
            ui.window,
            ui.clip,
            vec![Paint::Gradient {
                rect: hue,
                rounding: CornerRadius::all(4.0),
                colors: (0..2)
                    .flat_map(|_| {
                        (0..49).map(|column| from_hsv([column as f32 / 48.0, 1.0, 1.0], 255))
                    })
                    .collect(),
                columns: 49,
                rows: 2,
            }],
        );
        let point = palette.min + palette.size() * Vec2::new(state.hsv[1], 1.0 - state.hsv[2]);
        let thumb = Rect::from_min_size(
            Vec2::new(hue.min.x + width * state.hsv[0] - 2.0, hue.min.y - 2.0),
            Vec2::new(4.0, 18.0),
        );
        ui.context.paint(
            id.with("markers"),
            ui.window,
            ui.clip,
            vec![
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
                rounded(
                    palette,
                    Color::TRANSPARENT,
                    4.0,
                    if ui.context.focus_visible(id.with("palette")) {
                        style.focus_border
                    } else {
                        Border::NONE
                    },
                ),
                rounded(
                    hue,
                    Color::TRANSPARENT,
                    4.0,
                    if ui.context.focus_visible(id.with("hue")) {
                        style.focus_border
                    } else {
                        Border::NONE
                    },
                ),
            ],
        );
        let (blur, filter) = ui.control_blur(None);
        for (field, field_id) in field_ids.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                origin + Vec2::new(width * field as f32 / 4.0, 150.0),
                Vec2::new((width / 4.0 - 5.0).max(0.0), style.text_edit_height),
            );
            hit(ui, field_id, rect, self.enabled, HitAction::TextEdit);
            let field_response = ui.response(field_id, rect, self.enabled);
            response.has_focus |= field_response.has_focus;
            response.focus_visible |= field_response.focus_visible;
            response.pressed |= field_response.pressed;
            let edit = state.edit.as_ref().filter(|edit| edit.field == field);
            let text =
                edit.map_or_else(|| field_text(*self.color, field), |edit| edit.text.clone());
            let prefix = if field == 3 {
                ""
            } else {
                ["R ", "G ", "B "][field]
            };
            let size = super::font_size(style.text_edit_font_size);
            let height = ui.context.measure_text("", size, f32::INFINITY).y;
            let cursor_position = Vec2::new(
                rect.min.x + style.text_edit_padding.left,
                rect.center().y - height * 0.5,
            );
            let rendered = format!("{prefix}{text}");
            let position =
                cursor_position + Vec2::new(0.0, ui.context.centered_line_offset(&rendered, size));
            ui.context.paint_blur(
                field_id.with("blur"),
                ui.window,
                ui.clip,
                crate::Blur::new(rect).radius(filter).corner_radius(4.0),
            );
            let preset = self
                .hover_style
                .or(ui.hover_style)
                .unwrap_or(style.hover_style);
            let fill = if edit.is_some_and(|edit| !edit.buffer.selection().is_empty()) {
                style.button_hovered
            } else {
                style.button_fill
            };
            let hover = ui.animate_hover(
                field_response,
                preset,
                super::appearance::Appearance::new(
                    fill,
                    if field_response.focus_visible {
                        style.focus_border
                    } else {
                        style.border
                    },
                    if self.enabled {
                        style.text_color
                    } else {
                        style.muted_text
                    },
                ),
                style.button_hovered,
            );
            let mut body = Vec::new();
            hover.paint_shadow(rect, CornerRadius::all(4.0), &mut body);
            hover.paint_body(rect, CornerRadius::all(4.0), &style, blur, &mut body);
            ui.context
                .paint(field_id.with("body"), ui.window, ui.clip, body);
            let mut paint = vec![Paint::Text {
                text: rendered,
                position,
                size,
                wrap_width: f32::INFINITY,
                color: hover.text_color,
            }];
            if let Some(edit) = edit.filter(|edit| edit.buffer.selection().is_empty()) {
                let width = ui
                    .context
                    .measure_text(
                        &format!("{prefix}{}", &edit.text[..edit.buffer.cursor]),
                        size,
                        f32::INFINITY,
                    )
                    .x;
                paint.push(Paint::Shape(Shape::Line {
                    start: cursor_position + Vec2::new(width, 0.0),
                    end: cursor_position + Vec2::new(width, height),
                    width: 1.0,
                    color: style.text_color,
                }));
            }
            ui.context.paint(
                field_id.with("text"),
                ui.window,
                ui.clip.intersect(rect.shrink(3.0)),
                paint,
            );
        }
    }

    fn edit_field(&mut self, events: Vec<TextEditInput>, state: &mut ColorPickerState) {
        let field = state.edit.as_ref().unwrap().field;
        for event in events {
            if state.edit.is_none() {
                let text = field_text(*self.color, field);
                state.edit = Some(FieldEdit {
                    field,
                    buffer: EditBuffer {
                        cursor: text.len(),
                        anchor: 0,
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

impl Ui<'_> {
    #[track_caller]
    pub fn color_picker(&mut self, color: &mut Color, text: impl Into<String>) -> Response {
        self.add(ColorPicker::new(color, text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hsv_roundtrips_srgb_and_preserves_alpha() {
        for r in (0..=255).step_by(17) {
            for g in (0..=255).step_by(17) {
                for b in (0..=255).step_by(17) {
                    let color = Color::rgba(r, g, b, 73);
                    assert_eq!(from_hsv(to_hsv(color), 73), color);
                }
            }
        }
        assert_eq!(from_hsv([1.0, 1.0, 1.0], 255), Color::rgb(255, 0, 0));
    }
}
