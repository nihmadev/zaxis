use super::{Popup, Response, ScrollArea, ScrollStyle, TextEdit, Ui, Widget};
use crate::{
    context::{HitAction, HitRegion},
    CornerRadius, Easing, Id, Padding, Rect, TweenOptions, Vec2,
};
use std::{hash::Hash, panic::Location, time::Duration};
use winit::keyboard::KeyCode;

mod options;
mod paint;
mod show;
mod style;
pub use style::ComboBoxStyle;

/// An option's identity is independent of its label, value and position.
pub struct ComboBoxOption<T> {
    pub id: Id,
    pub value: T,
    pub label: String,
    pub enabled: bool,
}
impl<T> ComboBoxOption<T> {
    pub fn new(source: impl Hash, value: T, label: impl Into<String>) -> Self {
        Self {
            id: Id::new(source),
            value,
            label: label.into(),
            enabled: true,
        }
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }
}

/// Single selection with stable option IDs, keyboard navigation and optional filtering.
/// Missing selections are preserved in the model and displayed as the placeholder.
pub struct ComboBox<'a, T> {
    selected: &'a mut Option<T>,
    options: &'a [ComboBoxOption<T>],
    label: String,
    placeholder: String,
    id: Option<Id>,
    source: &'static Location<'static>,
    enabled: bool,
    filterable: bool,
    default_open: bool,
    style: Option<ComboBoxStyle>,
    width: Option<f32>,
    visible_rows: Option<usize>,
    duration: Option<Duration>,
    padding: Option<Padding>,
    row_height: Option<f32>,
    trigger_height: Option<f32>,
    row_gap: Option<f32>,
    label_gap: Option<f32>,
    rounding: Option<CornerRadius>,
    popup_padding: Option<Padding>,
    font_size: Option<f32>,
}
impl<'a, T> ComboBox<'a, T> {
    #[track_caller]
    pub fn new(selected: &'a mut Option<T>, options: &'a [ComboBoxOption<T>]) -> Self {
        Self {
            selected,
            options,
            label: String::new(),
            placeholder: "Select…".into(),
            id: None,
            source: Location::caller(),
            enabled: true,
            filterable: false,
            default_open: false,
            style: None,
            width: None,
            visible_rows: None,
            duration: None,
            padding: None,
            row_height: None,
            trigger_height: None,
            row_gap: None,
            label_gap: None,
            rounding: None,
            popup_padding: None,
            font_size: None,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }
    pub fn filterable(mut self, enabled: bool) -> Self {
        self.filterable = enabled;
        self
    }
    pub fn default_open(mut self, open: bool) -> Self {
        self.default_open = open;
        self
    }
    pub fn style(mut self, style: ComboBoxStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        assert!(width.is_finite() && width >= 0.0);
        self.width = Some(width);
        self
    }
    pub fn visible_rows(mut self, count: usize) -> Self {
        self.visible_rows = Some(count.max(1));
        self
    }
    pub fn animation_duration(mut self, duration: Duration) -> Self {
        self.duration = Some(duration);
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn row_height(mut self, height: f32) -> Self {
        assert!(height.is_finite() && height > 0.0);
        self.row_height = Some(height);
        self
    }
    pub fn trigger_height(mut self, height: f32) -> Self {
        assert!(height.is_finite() && height > 0.0);
        self.trigger_height = Some(height);
        self
    }
    pub fn row_gap(mut self, gap: f32) -> Self {
        assert!(gap.is_finite() && gap >= 0.0);
        self.row_gap = Some(gap);
        self
    }
    pub fn label_gap(mut self, gap: f32) -> Self {
        assert!(gap.is_finite() && gap >= 0.0);
        self.label_gap = Some(gap);
        self
    }
    pub fn rounding(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
    }
    pub fn popup_padding(mut self, padding: Padding) -> Self {
        self.popup_padding = Some(padding);
        self
    }
    pub fn font_size(mut self, size: f32) -> Self {
        assert!(size.is_finite() && size > 0.0);
        self.font_size = Some(size);
        self
    }
}

#[derive(Default)]
pub(crate) struct ComboBoxState {
    pub last_frame: u64,
    pub open: bool,
    pub active: Option<Id>,
    pub query: String,
    focus_filter: bool,
    active_row: Option<usize>,
    list_size: Vec2,
    options: options::OptionsState,
}

impl Ui<'_> {
    #[track_caller]
    pub fn combo_box<T: Clone + PartialEq>(
        &mut self,
        selected: &mut Option<T>,
        options: &[ComboBoxOption<T>],
    ) -> Response {
        self.add(ComboBox::new(selected, options))
    }
}
