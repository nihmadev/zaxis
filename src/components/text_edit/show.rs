use super::*;
impl Widget for TextEdit<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let id = self.exact_id.unwrap_or_else(|| match self.id {
            Some(id) => ui.scope.with(("text-edit", id)),
            None => ui.auto_id(("text-edit", self.source)),
        });
        let mut style = ui.style().clone();
        let mut component = style.text_edit;
        component.merge(self.style);
        if let Some(v) = component.cursor_width {
            style.text_edit_cursor_width = v.max(0.0);
        }
        if let Some(v) = component.blink_interval {
            style.text_edit_blink_interval = v;
        }
        let size = crate::components::font_size(
            self.size
                .or(component.font_size)
                .unwrap_or(style.text_edit_font_size),
        );
        let padding = self
            .padding
            .or(component.padding)
            .unwrap_or(style.text_edit_padding);
        let text_height = ui.context.measure_text("", size, f32::INFINITY).y;
        let rect = ui.allocate_space(Vec2::new(
            self.width
                .or(component.width)
                .unwrap_or(ui.layout.preferred_width.unwrap_or(style.text_edit_width))
                .max(0.0)
                .min(ui.available_width()),
            self.height
                .or(component.height)
                .unwrap_or(style.text_edit_height)
                .max(text_height + padding.size().y),
        ));
        let outer = padding.inset(rect);
        let prefix_width = ui
            .context
            .measure_text(&self.affixes.0, size, f32::INFINITY)
            .x;
        let suffix_width = ui
            .context
            .measure_text(&self.affixes.1, size, f32::INFINITY)
            .x;
        let inner = Rect::from_min_max(
            Vec2::new((outer.min.x + prefix_width).min(outer.max.x), outer.min.y),
            Vec2::new(
                (outer.max.x - suffix_width)
                    .max(outer.min.x + prefix_width)
                    .min(outer.max.x),
                outer.max.y,
            ),
        );
        let mut response = ui.response(id, rect, self.enabled);
        let mut state = ui
            .context
            .text_edits
            .remove(&id)
            .unwrap_or_else(|| TextEditState {
                buffer: EditBuffer {
                    cursor: self.text.len(),
                    anchor: self.text.len(),
                },
                scroll: 0.0,
                last_text: self.text.clone(),
                history: EditHistory::default(),
                word_drag: None,
                focused: false,
                blink_interval: style.text_edit_blink_interval,
                preedit: None,
            });
        if state.last_text != *self.text {
            state.buffer.clamp(self.text);
            state.preedit = None;
            state.history = EditHistory::default();
            state.word_drag = None;
        }
        if self.select_all {
            state.buffer.select_all(self.text);
        }
        let before = self.text.clone();
        let events = ui.context.take_text_edit_input(id);
        let activity = !events.is_empty() || state.focused != response.has_focus;
        response.lost_focus = state.focused && !response.has_focus;
        self.process_events(ui, &mut state, &mut response, events, inner, size);
        if !response.has_focus || self.read_only {
            state.preedit = None;
        }
        response.changed = before != *self.text;
        let selection = state.buffer.selection();
        let mut shown = display_line(self.text);
        let mut cursor = state.buffer.cursor;
        let composition = state.preedit.as_ref().map(|(text, caret)| {
            shown.replace_range(selection.clone(), text);
            cursor = selection.start + caret.map_or(text.len(), |(start, _)| start.min(text.len()));
            (selection.start, selection.start + text.len(), *caret)
        });
        let points = positions(ui, &shown, size);
        let caret_x = x_at(&points, cursor);
        let total = points.last().map_or(0.0, |p| p.1);
        let visible_width = (inner.size().x - style.text_edit_cursor_width).max(0.0);
        state.scroll = state.scroll.min((total - visible_width).max(0.0));
        if response.has_focus {
            if caret_x < state.scroll {
                state.scroll = caret_x;
            }
            if caret_x > state.scroll + visible_width {
                state.scroll = caret_x - visible_width;
            }
        }
        let position = Vec2::new(
            inner.min.x - state.scroll,
            inner.center().y - text_height * 0.5,
        );
        let clip = ui.clip.intersect(inner);
        let mut state_style =
            crate::components::theme::ControlState::from_response(response, false);
        state_style.focus = self.enabled && response.has_focus;
        let mut base = crate::components::appearance::Appearance::new(
            self.fill.unwrap_or(style.text_edit_fill),
            Border::NONE,
            self.text_color.unwrap_or(style.text_color),
        );
        base.rounding = self
            .rounding
            .or(component.rounding)
            .unwrap_or(style.text_edit_rounding);
        base.blur = 0.0;
        base.opacity = style.opacity;
        let explicit = self.hover_style.or(ui.hover_style);
        let preset = explicit.unwrap_or(crate::HoverStyle::fill(
            self.hovered_fill
                .or(self.fill)
                .unwrap_or(style.text_edit_hovered),
        ));
        let mut appearance = ui.animate_control(
            response,
            preset,
            explicit.is_some(),
            component.surface,
            state_style,
            base,
            self.hovered_fill
                .or(self.fill)
                .unwrap_or(style.text_edit_hovered),
        );
        if let Some(v) = self.rounding {
            appearance.rounding = v;
        }
        let color = crate::components::appearance::alpha(
            self.text_color.unwrap_or(appearance.text_color),
            appearance.opacity,
        );
        let caret = component.caret.unwrap_or(color);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if self.enabled {
                HitAction::TextEdit
            } else {
                HitAction::Block
            },
        });
        let (_, filter) = ui.resolved_blur(appearance.blur, component.surface.has_blur_override());
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(rect)
                .radius(filter)
                .corner_radius(appearance.rounding),
        );
        let mut body = Vec::new();
        appearance.paint_shadow(rect, appearance.rounding, &mut body);
        appearance.paint_body(
            rect,
            appearance.rounding,
            &style,
            appearance.blur,
            &mut body,
        );
        ui.context.paint(id.with("body"), ui.window, ui.clip, body);
        let mut paint = Vec::new();
        if response.has_focus && composition.is_none() && !selection.is_empty() {
            let a = x_at(&points, selection.start);
            let b = x_at(&points, selection.end);
            paint.push(Paint::Shape(
                Shape::rect(
                    Rect::from_min_size(
                        position + Vec2::new(a, 0.0),
                        Vec2::new((b - a).max(0.0), text_height),
                    ),
                    self.selection_color
                        .or(component.selection)
                        .unwrap_or(style.text_edit_selection),
                )
                .into(),
            ));
        }
        let placeholder = shown.is_empty();
        let rendered = if placeholder {
            display_line(&self.placeholder)
        } else {
            shown
        };
        let text_position =
            position + Vec2::new(0.0, ui.context.centered_line_offset(&rendered, size));
        paint.push(Paint::Text {
            text: rendered,
            position: text_position,
            size,
            wrap_width: f32::INFINITY,
            color: if placeholder {
                self.placeholder_color
                    .or(component.placeholder)
                    .unwrap_or(style.text_edit_placeholder)
            } else {
                color
            },
        });
        let mut selected_paint = None;
        if response.has_focus && composition.is_none() && !selection.is_empty() && !placeholder {
            if let Some(foreground) = component.selection_foreground {
                let a = x_at(&points, selection.start);
                let b = x_at(&points, selection.end);
                let selected_clip = Rect::from_min_size(
                    position + Vec2::new(a, 0.0),
                    Vec2::new((b - a).max(0.0), text_height),
                );
                selected_paint = Some((
                    clip.intersect(selected_clip),
                    vec![Paint::Text {
                        text: display_line(self.text),
                        position: text_position,
                        size,
                        wrap_width: f32::INFINITY,
                        color: foreground,
                    }],
                ));
            }
        }
        if let Some((start, end, Some((a, b)))) = composition {
            let a = x_at(&points, start + a.min(end - start));
            let b = x_at(&points, start + b.min(end - start));
            paint.insert(
                0,
                Paint::Shape(
                    Shape::rect(
                        Rect::from_min_size(
                            position + Vec2::new(a.min(b), 0.0),
                            Vec2::new((b - a).abs(), text_height),
                        ),
                        self.selection_color
                            .or(component.selection)
                            .unwrap_or(style.text_edit_selection),
                    )
                    .into(),
                ),
            );
        }
        if let Some((start, end, _)) = composition {
            paint.push(line(
                position
                    + Vec2::new(
                        x_at(&points, start),
                        text_height - style.text_edit_cursor_width,
                    ),
                position
                    + Vec2::new(
                        x_at(&points, end),
                        text_height - style.text_edit_cursor_width,
                    ),
                style.text_edit_cursor_width,
                color,
            ));
        }
        if response.has_focus {
            let interval = style.text_edit_blink_interval;
            let mut show = true;
            if !interval.is_zero() && composition.is_none() && !clip.is_empty() {
                let blink_id = id.with("cursor-blink");
                let create = || {
                    crate::Procedural::new(true, move |elapsed: Duration| {
                        let show = (elapsed.as_nanos() / interval.as_nanos()).is_multiple_of(2);
                        let remainder = Duration::new(
                            ((elapsed.as_nanos() % interval.as_nanos()) / 1_000_000_000) as u64,
                            ((elapsed.as_nanos() % interval.as_nanos()) % 1_000_000_000) as u32,
                        );
                        crate::AnimationSample::after(show, interval - remainder)
                    })
                };
                let options = crate::AnimationOptions { decorative: false };
                if activity || state.blink_interval != interval {
                    ui.context
                        .restart_animation_with(blink_id, create(), options);
                }
                show = ui.context.animate_with(blink_id, options, create).value;
            }
            if show && composition.is_none_or(|(_, _, caret)| caret.is_some()) {
                paint.push(line(
                    position + Vec2::new(caret_x, 0.0),
                    position + Vec2::new(caret_x, text_height),
                    style.text_edit_cursor_width,
                    caret,
                ));
            }
            if !self.read_only && !clip.is_empty() {
                ui.context.set_ime_area(
                    ui.window,
                    Rect::from_min_size(
                        Vec2::new(
                            (position.x + caret_x).clamp(inner.min.x, inner.max.x),
                            position.y,
                        ),
                        Vec2::new(style.text_edit_cursor_width, text_height),
                    )
                    .intersect(clip),
                );
            }
        }
        ui.context.paint(id.with("text"), ui.window, clip, paint);
        if let Some((clip, paint)) = selected_paint {
            ui.context
                .paint(id.with("selected-text"), ui.window, clip, paint);
        }
        self.paint_affixes(ui, id, outer, position, size, suffix_width, color);
        state.focused = response.has_focus;
        state.blink_interval = style.text_edit_blink_interval;
        state.last_text.clone_from(self.text);
        ui.context.text_edits.insert(id, state);
        response
    }
}
