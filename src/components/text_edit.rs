use super::{
    edit_buffer::{boundaries, word_at, EditBuffer},
    edit_history::EditHistory,
    Response, Ui, Widget,
};
use crate::{
    context::{HitAction, HitRegion, Paint, TextEditInput},
    Border, Color, CornerRadius, Id, Padding, Rect, Shape, Vec2,
};
use std::{hash::Hash, panic::Location, time::Duration};
use unicode_segmentation::UnicodeSegmentation;
use winit::keyboard::KeyCode;

mod events;
mod geometry;
use geometry::{display_line, line, positions, single_line, x_at};
pub(crate) type EditEventHandler<'a> = dyn FnMut(&mut String, &TextEditInput) -> bool + 'a;

/// A single-line editor bound directly to an application-owned string.
pub struct TextEdit<'a> {
    text: &'a mut String,
    placeholder: String,
    id: Option<Id>,
    source: &'static Location<'static>,
    enabled: bool,
    read_only: bool,
    width: Option<f32>,
    height: Option<f32>,
    size: Option<f32>,
    padding: Option<Padding>,
    rounding: Option<CornerRadius>,
    fill: Option<Color>,
    pub(crate) hovered_fill: Option<Color>,
    text_color: Option<Color>,
    placeholder_color: Option<Color>,
    selection_color: Option<Color>,
    pub(crate) event_handler: Option<&'a mut EditEventHandler<'a>>,
    pub(crate) exact_id: Option<Id>,
    pub(crate) affixes: (String, String),
    pub(crate) select_all: bool,
}

impl<'a> TextEdit<'a> {
    #[track_caller]
    pub fn new(text: &'a mut String) -> Self {
        Self {
            text,
            placeholder: String::new(),
            id: None,
            source: Location::caller(),
            enabled: true,
            read_only: false,
            width: None,
            height: None,
            size: None,
            padding: None,
            rounding: None,
            fill: None,
            hovered_fill: None,
            text_color: None,
            placeholder_color: None,
            selection_color: None,
            event_handler: None,
            exact_id: None,
            affixes: (String::new(), String::new()),
            select_all: false,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = value.into();
        self
    }
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }
    pub fn disabled(self, value: bool) -> Self {
        self.enabled(!value)
    }
    /// Read-only fields retain navigation, selection and copying.
    pub fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
    pub fn width(mut self, value: f32) -> Self {
        assert!(value.is_finite() && value > 0.0);
        self.width = Some(value);
        self
    }
    pub fn height(mut self, value: f32) -> Self {
        assert!(value.is_finite() && value > 0.0);
        self.height = Some(value);
        self
    }
    pub fn font_size(mut self, value: f32) -> Self {
        assert!(value.is_finite() && value > 0.0);
        self.size = Some(value);
        self
    }
    pub fn padding(mut self, value: Padding) -> Self {
        self.padding = Some(value);
        self
    }
    pub fn rounding(mut self, value: CornerRadius) -> Self {
        self.rounding = Some(value);
        self
    }
    pub fn fill(mut self, value: Color) -> Self {
        self.fill = Some(value);
        self
    }
    pub fn text_color(mut self, value: Color) -> Self {
        self.text_color = Some(value);
        self
    }
    pub fn placeholder_color(mut self, value: Color) -> Self {
        self.placeholder_color = Some(value);
        self
    }
    pub fn selection_color(mut self, value: Color) -> Self {
        self.selection_color = Some(value);
        self
    }
}

pub(crate) struct TextEditState {
    pub(crate) buffer: EditBuffer,
    pub(crate) scroll: f32,
    last_text: String,
    history: EditHistory,
    word_drag: Option<std::ops::Range<usize>>,
    focused: bool,
    blink_interval: Duration,
    pub(crate) preedit: Option<(String, Option<(usize, usize)>)>,
}

impl Widget for TextEdit<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let id = self.exact_id.unwrap_or_else(|| match self.id {
            Some(id) => ui.scope.with(("text-edit", id)),
            None => ui.auto_id(("text-edit", self.source)),
        });
        let style = ui.style().clone();
        let size = super::font_size(self.size.unwrap_or(style.text_edit_font_size));
        let padding = self.padding.unwrap_or(style.text_edit_padding);
        let text_height = ui.context.measure_text("", size, f32::INFINITY).y;
        let rect = ui.allocate_space(Vec2::new(
            self.width
                .unwrap_or(ui.layout.preferred_width.unwrap_or(style.text_edit_width))
                .max(0.0)
                .min(ui.available_width()),
            self.height
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
        let color = self.text_color.unwrap_or(if self.enabled {
            style.text_color
        } else {
            style.muted_text
        });
        let rounding = self.rounding.unwrap_or(style.text_edit_rounding);
        let target_fill = if response.hovered && self.enabled {
            self.hovered_fill
                .or(self.fill)
                .unwrap_or(style.text_edit_hovered)
        } else {
            self.fill.unwrap_or(style.text_edit_fill)
        };
        let fill = ui.transition_property(
            id.with("fill"),
            rect,
            target_fill,
            style.motion.hover.clone(),
        );
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
        ui.context.paint(
            id.with("body"),
            ui.window,
            ui.clip,
            vec![Paint::Shape(
                Shape::rect(rect, fill)
                    .corner_radius(rounding)
                    .border(if response.has_focus {
                        style.focus_border
                    } else {
                        Border::NONE
                    })
                    .into(),
            )],
        );
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
                    self.selection_color.unwrap_or(style.text_edit_selection),
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
                    .unwrap_or(style.text_edit_placeholder)
            } else {
                color
            },
        });
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
                        self.selection_color.unwrap_or(style.text_edit_selection),
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
                    color,
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
        self.paint_affixes(ui, id, outer, position, size, suffix_width, color);
        state.focused = response.has_focus;
        state.blink_interval = style.text_edit_blink_interval;
        state.last_text.clone_from(self.text);
        ui.context.text_edits.insert(id, state);
        response
    }
}

impl Ui<'_> {
    #[track_caller]
    pub fn text_edit(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::new(text))
    }
}
