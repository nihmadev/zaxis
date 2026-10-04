use super::*;
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
        let mut style = ui.style().clone();
        style.color_picker.merge(self.style);
        let component = style.color_picker;
        let row_height = component.row_height.unwrap_or(24.0);
        let editor_height = self.editor_height(&style);
        let width = self
            .width
            .or(component.width)
            .unwrap_or(300.0)
            .min(ui.available_width());
        let row = Rect::from_min_size(ui.layout.cursor, Vec2::new(width, row_height));
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
        let angle = ui.transition_property(
            id.with("chevron-angle"),
            row,
            if open {
                std::f32::consts::FRAC_PI_2
            } else {
                0.0
            },
            style.motion.expand.clone(),
        );
        let mut rect = ui.allocate_space(Vec2::new(width, row_height));
        response.rect = rect;
        hit(ui, id, row, self.enabled, HitAction::Activate);
        // Content is inset from the hover surface; the disclosure chevron leads the
        // label and the color swatch is the only trailing element.
        let inset = 8.0_f32.min(width * 0.25);
        let swatch_size = Vec2::new((width - 2.0 * inset).clamp(0.0, 28.0), 16.0);
        let swatch = Rect::from_min_size(
            Vec2::new(
                (row.max.x - inset - swatch_size.x).max(row.min.x),
                row.center().y - swatch_size.y * 0.5,
            ),
            swatch_size,
        );
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let mut row_response = response;
        row_response.rect = row;
        let mut base = crate::components::appearance::Appearance::new(
            Color::TRANSPARENT,
            Border::NONE,
            style.text_color,
        );
        base.rounding = component.rounding.unwrap_or(CornerRadius::all(4.0));
        base.blur = 0.0;
        base.opacity = style.opacity;
        let row_hover = ui.animate_control(
            row_response,
            preset,
            self.hover_style.or(ui.hover_style).is_some(),
            component.body,
            crate::ControlState::from_response(row_response, false),
            base,
            style.button_hovered,
        );
        let mut row_paint = Vec::new();
        row_hover.paint_shadow(row, row_hover.rounding, &mut row_paint);
        row_hover.paint_body(
            row,
            row_hover.rounding,
            &style,
            row_hover.blur,
            &mut row_paint,
        );
        let (_, filter) = ui.resolved_blur(row_hover.blur, component.body.has_blur_override());
        ui.context.paint_blur(
            id.with("row-blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(row)
                .radius(filter)
                .corner_radius(row_hover.rounding),
        );
        ui.context
            .paint(id.with("row"), ui.window, ui.clip, row_paint);

        // Floating panels are windows below a modal; inside one the editor expands inline.
        let picker_type = if ui.context.building_modal() {
            ColorPickerType::Internal
        } else {
            self.picker_type
        };
        if picker_type == ColorPickerType::Internal || open {
            match picker_type {
                ColorPickerType::Internal => {
                    let enabled = self.enabled;
                    self.enabled &= open;
                    let reveal_id = ui.scope.with(("reveal", Id::new(id)));
                    if open && !ui.context.effect_states.contains_key(&reveal_id) {
                        // Preserve the existing first-appearance snap for default_open.
                        ui.context.transition(
                            reveal_id,
                            editor_height,
                            style.motion.expand.clone(),
                        );
                    }
                    ui.layout.cursor.y = row.max.y;
                    ui.reveal(id, open, |ui| {
                        let origin =
                            ui.layout.cursor + Vec2::new(0.0, component.gap.unwrap_or(10.0));
                        self.editor(ui, id, origin, width, &mut state, &mut response);
                        ui.allocate_space(Vec2::new(width, editor_height));
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
                        self.width.or(component.width).unwrap_or(300.0).max(260.0) + 24.0,
                        style.title_height.max(24.0) + editor_height + 4.0,
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
                        .effective_style(style.clone())
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
                                crate::components::appearance::Appearance::new(
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
                            hover.paint_shadow(
                                close,
                                component.rounding.unwrap_or(CornerRadius::all(4.0)),
                                &mut paint,
                            );
                            hover.paint_body(
                                close,
                                component.rounding.unwrap_or(CornerRadius::all(4.0)),
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
        // Preserve the actual selected color when the row uses a hover gradient.
        let swatch_paint = vec![rounded(
            swatch,
            *self.color,
            component.rounding.unwrap_or(CornerRadius::all(4.0)),
            style.border,
        )];
        ui.context
            .paint(id.with("swatch"), ui.window, ui.clip, swatch_paint);
        let label = visible_label(&self.text);
        let label_size = crate::components::font_size(style.font_size);
        let weight = style.typography.weights.control;
        let label_height = ui
            .context
            .measure_text(label, label_size, weight, f32::INFINITY)
            .y;
        let optical = ui.context.centered_line_offset(label, label_size, weight);
        let label_x = row.min.x + inset + 20.0;
        ui.context.paint(
            id.with("caption"),
            ui.window,
            ui.clip.intersect(Rect::from_min_max(
                row.min,
                Vec2::new((swatch.min.x - 8.0).max(row.min.x), row.max.y),
            )),
            vec![Paint::Text {
                text: label.to_owned(),
                position: Vec2::new(label_x, row.center().y - label_height * 0.5 + optical),
                size: label_size,
                weight,
                wrap_width: f32::INFINITY,
                color: row_hover.text_color,
            }],
        );
        let center = Vec2::new(row.min.x + inset + 6.0, row.center().y);
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
                    start: rotate(Vec2::new(-2.0, -4.0)),
                    end: rotate(Vec2::new(2.0, 0.0)),
                    width: 1.5,
                    color: row_hover.text_color,
                }),
                Paint::Shape(Shape::Line {
                    start: rotate(Vec2::new(2.0, 0.0)),
                    end: rotate(Vec2::new(-2.0, 4.0)),
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
                component.rounding.unwrap_or(CornerRadius::all(4.0)),
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
