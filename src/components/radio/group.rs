use std::hash::Hash;

use super::super::{
    sanitize,
    theme::{painter::PaintHook, ControlPaint, PaintMode, Painter, RadioStyle},
    SemanticStatus, Ui,
};
use super::options::{Config, RadioLayout, RadioOption, RadioSize};
use crate::{Id, Response};

/// A list of mutually exclusive choices. It edits an application-owned `&mut T`; options are
/// matched by value and identified by the value's hash. Use `Option<U>` as `T` for a group
/// that may select nothing: `(Some(x), "label")` options never match `None`.
///
/// Keyboard: the group is a single Tab stop. Tab lands on the selected option (the first
/// enabled one when nothing is selected); arrows (by [`RadioLayout`]) move the focus to the
/// next enabled option, wrapping around, and by default select it; Home/End jump to the ends.
/// With [`focus_follows_selection(false)`](Self::focus_follows_selection) arrows only move the
/// focus and Space/Enter select. A click selects on release over the same row. The
/// [`Response`] reports `changed()` once per user action; assigning the value from outside
/// never does, a value that matches no option selects nothing and is left untouched, and a
/// selected disabled option stays selected. Empty and duplicate options are reported as
/// invalid values.
///
/// ```no_run
/// # fn view(ui: &mut zaxis::Ui<'_>) {
/// # #[derive(PartialEq, Hash)] enum Level { Low, High }
/// # let mut level = Level::Low;
/// ui.radio_group(&mut level, [(Level::Low, "Low"), (Level::High, "High")]);
/// # }
/// ```
pub struct RadioGroup<'a, T> {
    pub(super) selected: &'a mut T,
    pub(super) options: Vec<RadioOption<T>>,
    pub(super) cfg: Config,
    pub(super) painter: Option<PaintHook<'a>>,
}

impl<'a, T: PartialEq + Hash> RadioGroup<'a, T> {
    /// `(value, label)`, `(value, label, description)` tuples or [`RadioOption`]s.
    pub fn new<O: Into<RadioOption<T>>>(
        selected: &'a mut T,
        options: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            cfg: Config::default(),
            painter: None,
        }
    }

    /// Identity of the group among its siblings; by default the call order.
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.cfg.id = Some(Id::new(source));
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.cfg.enabled = enabled;
        self
    }
    /// Validation state tinting the rings; a [`Field`](crate::Field) status is inherited when
    /// not set.
    pub fn status(mut self, status: SemanticStatus) -> Self {
        self.cfg.status = status;
        self
    }
    pub fn layout(mut self, layout: RadioLayout) -> Self {
        self.cfg.layout = layout;
        self
    }
    pub fn horizontal(self) -> Self {
        self.layout(RadioLayout::Horizontal)
    }
    /// A grid with `columns` columns (row-major). Zero is reported and ignored.
    #[track_caller]
    pub fn columns(self, columns: usize) -> Self {
        if columns == 0 {
            crate::context::invalid_value("RadioGroup::columns", "expected >= 1; ignored".into());
            return self;
        }
        self.layout(RadioLayout::Grid(columns))
    }
    pub fn size(mut self, size: RadioSize) -> Self {
        self.cfg.size = size;
        self
    }
    /// Whether arrow keys select together with moving the focus; `true` by default.
    pub fn focus_follows_selection(mut self, follow: bool) -> Self {
        self.cfg.follow_focus = follow;
        self
    }
    /// Distance between rows, in logical pixels.
    #[track_caller]
    pub fn row_gap(mut self, gap: f32) -> Self {
        self.cfg.row_gap = sanitize::non_negative("RadioGroup::row_gap", gap).or(self.cfg.row_gap);
        self
    }
    /// Distance between columns (horizontal and grid layouts).
    #[track_caller]
    pub fn column_gap(mut self, gap: f32) -> Self {
        self.cfg.column_gap =
            sanitize::non_negative("RadioGroup::column_gap", gap).or(self.cfg.column_gap);
        self
    }
    /// Distance between the indicator and the label.
    #[track_caller]
    pub fn label_gap(mut self, gap: f32) -> Self {
        self.cfg.label_gap =
            sanitize::non_negative("RadioGroup::label_gap", gap).or(self.cfg.label_gap);
        self
    }
    /// Lower bound of a row height; `Style::control_height` by default.
    #[track_caller]
    pub fn min_row_height(mut self, height: f32) -> Self {
        self.cfg.min_row_height = sanitize::non_negative("RadioGroup::min_row_height", height)
            .or(self.cfg.min_row_height);
        self
    }
    /// Component style; builders override it, it overrides `Style::radio`.
    pub fn style(mut self, style: RadioStyle) -> Self {
        self.cfg.style = style;
        self
    }
    /// Called once per option and part: `RadioIndicator` and `RadioDot`.
    pub fn painter(
        mut self,
        mode: PaintMode,
        paint: impl Fn(&mut Painter<'_>, ControlPaint) + 'a,
    ) -> Self {
        self.painter = Some(PaintHook {
            mode,
            callback: Box::new(paint),
        });
        self
    }
}

impl Ui<'_> {
    /// Shorthand for `ui.add(RadioGroup::new(selected, options))`.
    pub fn radio_group<T, O>(
        &mut self,
        selected: &mut T,
        options: impl IntoIterator<Item = O>,
    ) -> Response
    where
        T: PartialEq + Hash,
        O: Into<RadioOption<T>>,
    {
        self.add(RadioGroup::new(selected, options))
    }

    /// One radio button on its own, like `selectable_value`: selects `value` when clicked.
    /// Every call is its own Tab stop; use [`Self::radio_group`] for arrow-key navigation.
    pub fn radio_value<T: PartialEq + Hash>(
        &mut self,
        selected: &mut T,
        value: T,
        label: impl Into<String>,
    ) -> Response {
        self.add(RadioGroup::new(selected, [(value, label.into())]))
    }
}
