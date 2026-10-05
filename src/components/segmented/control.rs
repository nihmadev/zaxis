use std::hash::Hash;

use super::super::{sanitize, SemanticStatus, Ui};
use super::options::{
    Config, SegmentOption, SegmentWidth, SegmentedOrientation, SegmentedSize, SegmentedVariant,
};
use crate::{components::theme::SegmentedStyle, CornerRadius, Id, Response};

/// A row (or column) of equal-weight choices on a rounded plate, with a raised thumb that
/// slides to the selected one. It edits an application-owned `&mut T`; the options are
/// matched by value, with an identity taken from the value's hash.
///
/// Sizing: segments are as wide as the widest content by default (see [`SegmentWidth`]). If
/// the row would overflow its container, segments shrink uniformly and labels are cut with
/// an ellipsis; when labels of segments that have icons would get unreadable, those labels
/// are hidden and only the icons stay (the label becomes the tooltip).
///
/// Keyboard: the control is a single Tab stop. Tab lands on the selected segment, the arrows
/// (by orientation) move focus and, by default, the selection; Home/End jump to the ends;
/// disabled segments are skipped. With
/// [`focus_follows_selection(false)`](Self::focus_follows_selection) arrows only move the
/// focus and Space/Enter select. The [`Response`] reports `changed()` once per user action;
/// assigning the value from outside never does. A value that matches no option selects
/// nothing and is left untouched.
///
/// ```no_run
/// # fn view(ui: &mut zaxis::Ui<'_>) {
/// # #[derive(PartialEq, Hash, Clone, Copy)] enum Mode { Table, Board }
/// # let mut mode = Mode::Table;
/// ui.segmented(&mut mode, [(Mode::Table, "Table"), (Mode::Board, "Board")]);
/// # }
/// ```
pub struct SegmentedControl<'a, T> {
    pub(super) selected: &'a mut T,
    pub(super) options: Vec<SegmentOption<T>>,
    pub(super) cfg: Config,
}

impl<'a, T: PartialEq + Hash + Clone> SegmentedControl<'a, T> {
    /// `(value, label)`, `(value, label, icon)` tuples or [`SegmentOption`]s.
    pub fn new<O: Into<SegmentOption<T>>>(
        selected: &'a mut T,
        options: impl IntoIterator<Item = O>,
    ) -> Self {
        Self {
            selected,
            options: options.into_iter().map(Into::into).collect(),
            cfg: Config::default(),
        }
    }

    /// Identity of the control among its siblings; by default the call order.
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.cfg.id = Some(Id::new(source));
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.cfg.enabled = enabled;
        self
    }
    /// Validation state; a [`Field`](crate::Field) status is inherited when not set.
    pub fn status(mut self, status: SemanticStatus) -> Self {
        self.cfg.status = status;
        self
    }
    pub fn orientation(mut self, orientation: SegmentedOrientation) -> Self {
        self.cfg.orientation = orientation;
        self
    }
    pub fn vertical(self) -> Self {
        self.orientation(SegmentedOrientation::Vertical)
    }
    /// [`SegmentedVariant::Outlined`] gives the Material 3 look; its defaults come from
    /// `Style::segmented_outlined`.
    pub fn variant(mut self, variant: SegmentedVariant) -> Self {
        self.cfg.variant = variant;
        self
    }
    pub fn size(mut self, size: SegmentedSize) -> Self {
        self.cfg.size = size;
        self
    }
    /// Segment widths; [`SegmentWidth::Equal`] by default. A non-finite or non-positive
    /// `Fixed` width is reported as an invalid value and ignored.
    #[track_caller]
    pub fn width(mut self, width: SegmentWidth) -> Self {
        match width {
            SegmentWidth::Fixed(w) => {
                if let Some(w) = sanitize::positive("SegmentedControl::width", w) {
                    self.cfg.width = SegmentWidth::Fixed(w);
                }
            }
            other => self.cfg.width = other,
        }
        self
    }
    /// Lower bound of a segment's width before shrinking, in logical pixels.
    #[track_caller]
    pub fn min_segment_width(mut self, width: f32) -> Self {
        self.cfg.min_width = sanitize::non_negative("SegmentedControl::min_segment_width", width)
            .or(self.cfg.min_width);
        self
    }
    /// Draw only the icons (labels become tooltips); segments without an icon keep their text.
    pub fn icon_only(mut self, icon_only: bool) -> Self {
        self.cfg.icon_only = icon_only;
        self
    }
    /// Whether arrow keys change the selection together with the focus; `true` by default.
    /// When `false`, Space and Enter select the focused segment.
    pub fn focus_follows_selection(mut self, follow: bool) -> Self {
        self.cfg.follow_focus = follow;
        self
    }
    /// Component style; builders below override it, it overrides `Style::segmented`.
    pub fn style(mut self, style: SegmentedStyle) -> Self {
        self.cfg.style = style;
        self
    }
    /// Radius of the plate; the thumb gets this minus the plate padding.
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.cfg.rounding = Some(radius.into());
        self
    }
}

impl Ui<'_> {
    /// Shorthand for `ui.add(SegmentedControl::new(selected, options))`.
    pub fn segmented<T, O>(
        &mut self,
        selected: &mut T,
        options: impl IntoIterator<Item = O>,
    ) -> Response
    where
        T: PartialEq + Hash + Clone,
        O: Into<SegmentOption<T>>,
    {
        self.add(SegmentedControl::new(selected, options))
    }
}
