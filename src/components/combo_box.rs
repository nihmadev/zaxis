use super::{Popup, Response, ScrollArea, ScrollStyle, TextEdit, Ui, Widget};
use crate::{
    context::{HitAction, HitRegion},
    CornerRadius, Easing, Id, Padding, Rect, TweenOptions, Vec2,
};
use std::{hash::Hash, panic::Location, time::Duration};
use winit::keyboard::KeyCode;

mod access;
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
    #[deprecated(
        note = "use `.enabled(!disabled)`; `enabled` is the one way to set a control's availability"
    )]
    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }
}

/// Borrowed options, or ones built from `(value, label)` pairs.
enum OptionList<'a, T> {
    Borrowed(&'a [ComboBoxOption<T>]),
    Owned(Vec<ComboBoxOption<T>>),
}
impl<T> std::ops::Deref for OptionList<'_, T> {
    type Target = [ComboBoxOption<T>];
    fn deref(&self) -> &[ComboBoxOption<T>] {
        match self {
            Self::Borrowed(options) => options,
            Self::Owned(options) => options,
        }
    }
}

/// Single selection with stable option IDs, keyboard navigation and optional filtering.
/// Missing selections are preserved in the model and displayed as the placeholder.
pub struct ComboBox<'a, T> {
    selected: &'a mut Option<T>,
    options: OptionList<'a, T>,
    label: String,
    placeholder: String,
    id: Option<Id>,
    source: &'static Location<'static>,
    enabled: bool,
    status: super::SemanticStatus,
    hover_style: Option<super::HoverStyle>,
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
        Self::with_options(selected, OptionList::Borrowed(options), Location::caller())
    }
    /// Options from `(value, label)` pairs, as for [`Ui::tab_bar`]: each option's id is
    /// derived from its value, so no [`ComboBoxOption`] list has to be built or kept.
    #[track_caller]
    pub fn from_pairs<L: Into<String>>(
        selected: &'a mut Option<T>,
        pairs: impl IntoIterator<Item = (T, L)>,
    ) -> Self
    where
        T: Hash,
    {
        let options = pairs
            .into_iter()
            .map(|(value, label)| ComboBoxOption {
                id: Id::new(&value),
                value,
                label: label.into(),
                enabled: true,
            })
            .collect();
        Self::with_options(selected, OptionList::Owned(options), Location::caller())
    }
    fn with_options(
        selected: &'a mut Option<T>,
        options: OptionList<'a, T>,
        source: &'static Location<'static>,
    ) -> Self {
        Self {
            selected,
            options,
            label: String::new(),
            placeholder: "Select…".into(),
            id: None,
            source,
            enabled: true,
            status: Default::default(),
            hover_style: None,
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
    #[deprecated(
        note = "use `.enabled(!disabled)`; `enabled` is the one way to set a control's availability"
    )]
    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }
    /// Validation state: the border and a soft ring take the status color. Fields inherit
    /// the status of an enclosing [`super::Field`] unless this is set.
    pub fn status(mut self, status: super::SemanticStatus) -> Self {
        self.status = status;
        self
    }
    pub fn hover_style(mut self, style: super::HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
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
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = super::sanitize::non_negative("ComboBox::width", width).or(self.width);
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
    #[track_caller]
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height =
            super::sanitize::positive("ComboBox::row_height", height).or(self.row_height);
        self
    }
    #[track_caller]
    pub fn trigger_height(mut self, height: f32) -> Self {
        self.trigger_height =
            super::sanitize::positive("ComboBox::trigger_height", height).or(self.trigger_height);
        self
    }
    #[track_caller]
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.row_gap = super::sanitize::non_negative("ComboBox::row_gap", gap).or(self.row_gap);
        self
    }
    #[track_caller]
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.label_gap =
            super::sanitize::non_negative("ComboBox::label_gap", gap).or(self.label_gap);
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
    }
    #[deprecated(
        note = "use `.corner_radius(..)`; one name for the corner radius of every component"
    )]
    pub fn rounding(self, radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius(radius)
    }
    pub fn popup_padding(mut self, padding: Padding) -> Self {
        self.popup_padding = Some(padding);
        self
    }
    #[track_caller]
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = super::sanitize::positive("ComboBox::font_size", size).or(self.font_size);
        self
    }
}

#[derive(Default)]
pub struct ComboBoxState {
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
    /// A combo box over `(value, label)` pairs bound directly to a value:
    /// `ui.combo_box_values(&mut mode, [(Mode::Fast, "Fast"), (Mode::Exact, "Exact")])`.
    /// Option ids derive from the values, so the values must be distinct.
    #[track_caller]
    pub fn combo_box_values<T: Clone + PartialEq + Hash, L: Into<String>>(
        &mut self,
        selected: &mut T,
        pairs: impl IntoIterator<Item = (T, L)>,
    ) -> Response {
        let mut current = Some(selected.clone());
        let response = self.add(ComboBox::from_pairs(&mut current, pairs));
        if let Some(value) = current {
            if value != *selected {
                *selected = value;
            }
        }
        response
    }
    #[track_caller]
    pub fn combo_box<T: Clone + PartialEq>(
        &mut self,
        selected: &mut Option<T>,
        options: &[ComboBoxOption<T>],
    ) -> Response {
        self.add(ComboBox::new(selected, options))
    }
}
