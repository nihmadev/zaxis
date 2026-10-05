//! The single-line layout: one unwrapped line scrolled horizontally inside the field.
use super::{
    blink::caret_visible,
    chrome::Look,
    events::{Geo, Outcome},
    text_input::Fingerprint,
    *,
};
use crate::{
    components::edit_buffer::{Pos, Surface},
    context::Context,
};

/// [`Surface`] of a single line: positions come from the shaped carets.
pub(super) struct LineSurface<'a> {
    pub ctx: &'a mut Context,
    pub size: f32,
    pub font: crate::text::TextFont,
}

impl Surface for LineSurface<'_> {
    fn line(&mut self, text: &str, _: Pos) -> (Range<usize>, bool) {
        (0..text.len(), false)
    }
    fn vertical(&mut self, _: &str, _: Pos, _: Option<f32>, _: i32) -> Option<(Pos, f32)> {
        None
    }
    fn hit(&mut self, text: &str, point: Vec2) -> (Pos, usize) {
        let points = positions(self.ctx, text, self.size, self.font);
        let nearest = points
            .iter()
            .min_by(|a, b| (a.1 - point.x).abs().total_cmp(&(b.1 - point.x).abs()))
            .map_or(0, |p| p.0);
        let cell = points
            .iter()
            .rev()
            .find(|(_, px)| *px <= point.x)
            .map_or(0, |p| p.0);
        (
            Pos {
                byte: nearest,
                upstream: false,
            },
            cell,
        )
    }
}

impl TextEdit<'_> {
    pub(super) fn show_single(mut self, ui: &mut Ui<'_>, id: Id, look: Look) -> Response {
        let Look {
            style,
            component,
            size,
            font,
            padding,
        } = &look;
        let (size, font, padding) = (*size, *font, *padding);
        let text_height = ui.context.measure_text("", size, font, f32::INFINITY).y;
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
            .measure_text(&self.affixes.0, size, font, f32::INFINITY)
            .x;
        let suffix_width = ui
            .context
            .measure_text(&self.affixes.1, size, font, f32::INFINITY)
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
            .text_fields
            .states
            .remove(&id)
            .unwrap_or_else(|| TextEditState::new(self.text, style.text_edit_blink_interval));
        let mut fingerprint = Fingerprint::of(self.text);
        if state.fingerprint != fingerprint {
            state.text_replaced(self.text);
        }
        if self.select_all {
            state.buffer.select_all(self.text);
        }
        let mut events = ui.context.take_text_edit_input(id);
        access::requests(ui.context, id, &mut events);
        let activity = !events.is_empty() || state.focused != response.has_focus;
        response.lost_focus = state.focused && !response.has_focus;
        let mut out = Outcome::default();
        let mut geo = Geo {
            doc: None,
            size,
            font,
            origin: Vec2::new(inner.min.x - state.scroll, 0.0),
            page: 1,
        };
        self.process_events(ui, &mut state, &mut out, events, &mut geo);
        response.lost_focus |= out.lost_focus;
        response.submitted |= out.submitted;
        if !response.has_focus || self.read_only {
            state.preedit = None;
        }
        response.changed = out.changed;
        if out.changed {
            fingerprint = Fingerprint::of(self.text);
        }
        let selection = state.buffer.selection();
        let mut shown = display_line(self.text);
        let mut cursor = state.buffer.cursor;
        let composition = state.preedit.as_ref().map(|(text, caret)| {
            shown.replace_range(selection.clone(), text);
            cursor = selection.start + caret.map_or(text.len(), |(start, _)| start.min(text.len()));
            (selection.start, selection.start + text.len(), *caret)
        });
        let points = positions(ui.context, &shown, size, font);
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
        if ui.context.a11y_on() {
            // The layout the carets above came from: the same cached shaping.
            let layout = ui.context.paragraph_layout(
                &display_line(&shown),
                size,
                font,
                f32::INFINITY,
                crate::text::DEFAULT_TAB,
            );
            let top = ui.context.centered_line_offset(&shown, size, font);
            let lines = access::Lines {
                text: access::line(&mut state.access, &shown, &layout),
                origin: position + Vec2::new(0.0, top) - rect.min,
                selection: match composition {
                    Some(_) => (cursor, cursor),
                    None => (state.buffer.anchor, state.buffer.cursor),
                },
            };
            self.describe(
                ui,
                id,
                rect,
                Some(lines),
                composition.map(|_| shown.as_str()),
            );
        }
        let chrome = self.paint_chrome(ui, id, rect, response, &look);
        let (color, caret) = (chrome.color, chrome.caret);
        let selection_fill = self.selection_fill(&look);
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
                    selection_fill,
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
            position + Vec2::new(0.0, ui.context.centered_line_offset(&rendered, size, font));
        paint.push(Paint::text(
            rendered,
            text_position,
            size,
            font,
            f32::INFINITY,
            crate::text::DEFAULT_TAB,
            if placeholder {
                self.placeholder_fill(&look)
            } else {
                color
            },
        ));
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
                    vec![Paint::text(
                        display_line(self.text),
                        text_position,
                        size,
                        font,
                        f32::INFINITY,
                        crate::text::DEFAULT_TAB,
                        foreground,
                    )],
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
                        selection_fill,
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
            let show = caret_visible(
                ui,
                id,
                &state,
                style.text_edit_blink_interval,
                activity,
                composition.is_some() || clip.is_empty(),
            );
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
        self.paint_affixes(ui, id, outer, position, size, font, suffix_width, color);
        state.focused = response.has_focus;
        state.blink_interval = style.text_edit_blink_interval;
        state.fingerprint = fingerprint;
        ui.context.text_fields.states.insert(id, state);
        response
    }
}
