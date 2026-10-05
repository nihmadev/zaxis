use super::*;

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

impl ColorPicker<'_> {
    pub(super) fn editor(
        &mut self,
        ui: &mut Ui<'_>,
        id: Id,
        origin: Vec2,
        width: f32,
        state: &mut ColorPickerState,
        response: &mut Response,
    ) {
        let mut style = ui.style().clone();
        style.color_picker.merge(self.style);
        let component = style.color_picker;
        let palette_height = component.palette_height.unwrap_or(112.0);
        let hue_height = component.hue_height.unwrap_or(14.0);
        let gap = component.gap.unwrap_or(10.0);
        let palette = Rect::from_min_size(origin, Vec2::new(width, palette_height));
        let hue = Rect::from_min_size(
            origin + Vec2::new(0.0, palette_height + gap),
            Vec2::new(width, hue_height),
        );
        let field_ids = std::array::from_fn::<_, 4, _>(|field| id.with(("field", field)));
        let focused = field_ids
            .iter()
            .position(|field| self.enabled && ui.context.has_focus(*field));
        access::field_requests(ui, &field_ids, self.enabled, state, self.color);
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
                            ..Default::default()
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
                        ..Default::default()
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
                adjust(&mut state.hsv, rect, is_hue, event);
                *self.color = from_hsv(state.hsv, self.color.0[3]);
            }
            let surface = (control_id, rect, is_hue);
            if access::sliders(ui, surface, self.enabled, state.open, &mut state.hsv) {
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
                rounding: component.rounding.unwrap_or(CornerRadius::all(4.0)),
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
                rounding: component.rounding.unwrap_or(CornerRadius::all(4.0)),
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
            Vec2::new(4.0, hue_height + 4.0),
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
                    component.rounding.unwrap_or(CornerRadius::all(4.0)),
                    if ui.context.focus_visible(id.with("palette")) {
                        style.focus_border
                    } else {
                        Border::NONE
                    },
                ),
                rounded(
                    hue,
                    Color::TRANSPARENT,
                    component.rounding.unwrap_or(CornerRadius::all(4.0)),
                    if ui.context.focus_visible(id.with("hue")) {
                        style.focus_border
                    } else {
                        Border::NONE
                    },
                ),
            ],
        );
        let (blur, _) = ui.control_blur(None);
        for (field, field_id) in field_ids.into_iter().enumerate() {
            let rect = Rect::from_min_size(
                origin
                    + Vec2::new(
                        width * field as f32 / 4.0,
                        palette_height + hue_height + gap * 2.0 + 4.0,
                    ),
                Vec2::new(
                    (width / 4.0 - component.field_gap.unwrap_or(5.0)).max(0.0),
                    style.text_edit_height,
                ),
            );
            hit(ui, field_id, rect, self.enabled, HitAction::TextEdit);
            let field_response = ui.response(field_id, rect, self.enabled);
            response.has_focus |= field_response.has_focus;
            response.focus_visible |= field_response.focus_visible;
            response.pressed |= field_response.pressed;
            let edit = state.edit.as_ref().filter(|edit| edit.field == field);
            let text =
                edit.map_or_else(|| field_text(*self.color, field), |edit| edit.text.clone());
            if state.open {
                access::field(ui, field_id, rect, field, &text, self.enabled);
            }
            let prefix = if field == 3 {
                ""
            } else {
                ["R ", "G ", "B "][field]
            };
            let size = crate::components::font_size(style.text_edit_font_size);
            let weight = style.typography.weights.body;
            let height = ui.context.measure_text("", size, weight, f32::INFINITY).y;
            let cursor_position = Vec2::new(
                rect.min.x + style.text_edit_padding.left,
                rect.center().y - height * 0.5,
            );
            let rendered = format!("{prefix}{text}");
            let position = cursor_position
                + Vec2::new(
                    0.0,
                    ui.context.centered_line_offset(&rendered, size, weight),
                );
            let preset = self
                .hover_style
                .or(ui.hover_style)
                .unwrap_or(style.hover_style);
            let fill = if edit.is_some_and(|edit| !edit.buffer.selection().is_empty()) {
                style.text_edit_selection
            } else {
                style.text_edit_fill
            };
            let selected = edit.is_some_and(|edit| !edit.buffer.selection().is_empty());
            let mut base = crate::components::appearance::Appearance::new(
                fill,
                style.border,
                if selected {
                    style.selected_text
                } else {
                    style.text_color
                },
            );
            base.rounding = component.rounding.unwrap_or(style.text_edit_rounding);
            base.blur = blur;
            base.opacity = style.opacity;
            let hover = ui.animate_control(
                field_response,
                preset,
                self.hover_style.or(ui.hover_style).is_some(),
                component.field,
                crate::ControlState::from_response(field_response, selected),
                base,
                style.text_edit_hovered,
            );
            let (_, filter) = ui.resolved_blur(hover.blur, component.field.has_blur_override());
            ui.context.paint_blur(
                field_id.with("blur"),
                ui.window,
                ui.clip,
                crate::Blur::new(rect)
                    .radius(filter)
                    .corner_radius(hover.rounding),
            );
            let mut body = Vec::new();
            hover.paint_shadow(rect, hover.rounding, &mut body);
            hover.paint_body(rect, hover.rounding, &style, hover.blur, &mut body);
            ui.context
                .paint(field_id.with("body"), ui.window, ui.clip, body);
            let mut paint = vec![Paint::Text {
                text: rendered,
                position,
                size,
                weight,
                wrap_width: f32::INFINITY,
                color: hover.text_color,
            }];
            if let Some(edit) = edit.filter(|edit| edit.buffer.selection().is_empty()) {
                let width = ui
                    .context
                    .measure_text(
                        &format!("{prefix}{}", &edit.text[..edit.buffer.cursor]),
                        size,
                        weight,
                        f32::INFINITY,
                    )
                    .x;
                paint.push(Paint::Shape(Shape::Line {
                    start: cursor_position + Vec2::new(width, 0.0),
                    end: cursor_position + Vec2::new(width, height),
                    width: style.text_edit_cursor_width,
                    color: style.text_edit.caret.unwrap_or(style.text_color),
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
}
