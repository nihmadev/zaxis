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
    style: super::theme::TextEditStyle,
    hover_style: Option<super::HoverStyle>,
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
            style: Default::default(),
            hover_style: None,
            event_handler: None,
            exact_id: None,
            affixes: (String::new(), String::new()),
            select_all: false,
        }
    }
    pub fn style(mut self, style: super::theme::TextEditStyle) -> Self {
        self.style = style;
        self
    }
    pub fn caret_color(mut self, color: Color) -> Self {
        self.style.caret = Some(color);
        self
    }
    pub fn hover_style(mut self, style: super::HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
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

mod show;

impl Ui<'_> {
    #[track_caller]
    pub fn text_edit(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::new(text))
    }
}
