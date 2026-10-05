use super::{
    edit_buffer::{boundaries, EditBuffer},
    edit_history::EditHistory,
    Response, Ui, Widget,
};
use crate::{
    context::{HitAction, HitRegion, Paint, TextEditInput},
    Border, Color, CornerRadius, Id, Padding, Rect, Shape, Vec2,
};
use std::{hash::Hash, ops::Range, panic::Location, time::Duration};
use text_input::Fingerprint;
use unicode_segmentation::UnicodeSegmentation;
use winit::keyboard::KeyCode;

mod access;
mod area;
mod area_paint;
mod blink;
mod chrome;
pub(crate) mod doc;
mod doc_layout;
mod doc_rich;
mod events;
mod geometry;
mod options;
mod scroll;
mod show;
mod single;
mod text_input;
use geometry::{display_line, line, positions, x_at};
use options::{AreaHeight, AreaOptions};
pub(crate) type EditEventHandler<'a> = dyn FnMut(&mut String, &TextEditInput) -> bool + 'a;

/// An editor bound directly to an application-owned string: a single line by default,
/// a scrolling multi-line text area after [`TextEdit::multiline`].
///
/// Both modes are one engine: the same buffer, undo history, clipboard, IME and mouse
/// handling. Only the layout differs (one line, or wrapped paragraphs) and the Enter rule
/// (submit, or insert a line break). Line breaks are stored as `\n`; `\r\n` and `\r` are
/// normalized when text is typed, pasted or composed.
pub struct TextEdit<'a> {
    text: &'a mut String,
    placeholder: String,
    id: Option<Id>,
    source: &'static Location<'static>,
    enabled: bool,
    status: super::SemanticStatus,
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
    /// Characters the field takes: typed, pasted, composed and assigned text keeps only
    /// these. For fields of a fixed alphabet (channel and hex fields of a color picker).
    pub(crate) accept: Option<fn(char) -> bool>,
    /// The field holds one value that is replaced rather than edited: gaining focus selects
    /// it whole (the press that gave focus keeps that selection instead of placing the
    /// caret), and so does Enter or Escape taken by the event handler.
    pub(crate) whole_value: bool,
    max_chars: Option<usize>,
    area: Option<AreaOptions>,
    family: Option<crate::TextFamily>,
    tabular: Option<bool>,
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
            status: Default::default(),
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
            accept: None,
            whole_value: false,
            max_chars: None,
            area: None,
            family: None,
            tabular: None,
        }
    }
    pub fn style(mut self, style: super::theme::TextEditStyle) -> Self {
        self.style = style;
        self
    }
    /// Edit in the monospace family: bundled JetBrains Mono by default. Every character of
    /// the main font is one cell wide, so columns, the caret, selection and IME rectangles
    /// use the same cell grid as `Text::monospace`. Tab width is [`Self::tab_size`] in cells.
    pub fn monospace(mut self) -> Self {
        self.family = Some(crate::TextFamily::Monospace);
        self
    }
    /// Choose the font family of this field; the default is the theme's (`Proportional`).
    pub fn family(mut self, family: crate::TextFamily) -> Self {
        self.family = Some(family);
        self
    }
    /// Tabular figures (OpenType `tnum`) for digits in the proportional family, when the font
    /// has them. Has no effect on the monospace family.
    pub fn tabular_numbers(mut self, tabular: bool) -> Self {
        self.tabular = Some(tabular);
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
    /// Validation state: the border and a soft ring take the status color. Fields inherit
    /// the status of an enclosing [`super::Field`] unless this is set.
    pub fn status(mut self, status: super::SemanticStatus) -> Self {
        self.status = status;
        self
    }
    #[deprecated(
        note = "use `.enabled(!disabled)`; `enabled` is the one way to set a control's availability"
    )]
    pub fn disabled(self, value: bool) -> Self {
        self.enabled(!value)
    }
    /// Read-only fields retain navigation, selection and copying.
    pub fn read_only(mut self, value: bool) -> Self {
        self.read_only = value;
        self
    }
    #[track_caller]
    pub fn width(mut self, value: f32) -> Self {
        self.width = super::sanitize::positive("TextEdit::width", value).or(self.width);
        self
    }
    /// Total height including padding. For a text area this fixes the height; the
    /// content scrolls inside it.
    #[track_caller]
    pub fn height(mut self, value: f32) -> Self {
        self.height = super::sanitize::positive("TextEdit::height", value).or(self.height);
        self
    }
    #[track_caller]
    pub fn font_size(mut self, value: f32) -> Self {
        self.size = super::sanitize::positive("TextEdit::font_size", value).or(self.size);
        self
    }
    pub fn padding(mut self, value: Padding) -> Self {
        self.padding = Some(value);
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
    }
    #[deprecated(
        note = "use `.corner_radius(..)`; one name for the corner radius of every component"
    )]
    pub fn rounding(self, value: CornerRadius) -> Self {
        self.corner_radius(value)
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
    /// Limit the text to `count` characters (Unicode scalar values). Typed, pasted and
    /// composed text is cut at a grapheme boundary to fit; text set by the application
    /// is never truncated.
    pub fn max_chars(mut self, count: usize) -> Self {
        self.max_chars = Some(count);
        self
    }
}

/// The paint of a field's body. Its state is retained while the body is painted.
pub(crate) fn body_id(id: Id) -> Id {
    id.with("body")
}

pub struct TextEditState {
    pub buffer: EditBuffer,
    pub scroll: f32,
    fingerprint: Fingerprint,
    history: EditHistory,
    word_drag: Option<Range<usize>>,
    /// The press that started `word_drag` selected whole paragraphs, not words.
    drag_paragraphs: bool,
    focused: bool,
    blink_interval: Duration,
    pub preedit: Option<(String, Option<(usize, usize)>)>,
    area: Option<Box<area::AreaState>>,
    /// The published line of a single-line field; see [`access`].
    access: access::LineCache,
}

impl Ui<'_> {
    #[track_caller]
    pub fn text_edit(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::new(text))
    }
    /// A multi-line text area with default settings; see [`TextEdit::multiline`].
    #[track_caller]
    pub fn text_area(&mut self, text: &mut String) -> Response {
        self.add(TextEdit::new(text).multiline())
    }
}
