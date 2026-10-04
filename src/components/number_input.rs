//! Typed numeric entry, sharing selection, clipboard, IME and undo with TextEdit.
use super::{Numeric, Response, Ui, Widget};
use crate::{Color, CornerRadius, Id, Padding};
use std::{hash::Hash, ops::RangeInclusive, panic::Location};

mod render;

/// Compact appearance and gesture defaults for both numeric controls.
#[derive(Clone, Debug, PartialEq)]
pub struct NumberStyle {
    pub surface: crate::ControlStyle,

    pub width: f32,
    pub height: f32,
    pub padding: Padding,
    pub rounding: CornerRadius,
    pub font_size: f32,
    pub fill: Color,
    pub hovered: Color,
    pub invalid: Color,
    /// Number of steps per logical pixel of horizontal motion.
    pub sensitivity: f64,
    pub shift_multiplier: f64,
    pub ctrl_multiplier: f64,
}
impl Default for NumberStyle {
    fn default() -> Self {
        Self {
            surface: Default::default(),

            width: 120.0,
            height: 28.0,
            padding: Padding::symmetric(8.0, 3.0),
            rounding: CornerRadius::all(5.0),
            font_size: 14.0,
            fill: Color::gray(55),
            hovered: Color::gray(61),
            invalid: Color::rgb(220, 94, 94),
            sensitivity: 0.1,
            shift_multiplier: 0.1,
            ctrl_multiplier: 10.0,
        }
    }
}

impl NumberStyle {
    /// Sizes and multipliers that are not finite and positive fall back to the defaults.
    #[track_caller]
    pub(crate) fn normalize(&mut self) {
        use super::sanitize::positive;
        let default = Self::default();
        self.width = positive("NumberStyle::width", self.width).unwrap_or(default.width);
        self.height = positive("NumberStyle::height", self.height).unwrap_or(default.height);
        self.font_size =
            positive("NumberStyle::font_size", self.font_size).unwrap_or(default.font_size);
        let multiplier = |what, value: f64, fallback: f64| {
            if value.is_finite() && value > 0.0 {
                value
            } else {
                crate::context::invalid_value(
                    what,
                    format!("expected a finite value > 0, got {value}; using {fallback}"),
                );
                fallback
            }
        };
        self.sensitivity = multiplier(
            "NumberStyle::sensitivity",
            self.sensitivity,
            default.sensitivity,
        );
        self.shift_multiplier = multiplier(
            "NumberStyle::shift_multiplier",
            self.shift_multiplier,
            default.shift_multiplier,
        );
        self.ctrl_multiplier = multiplier(
            "NumberStyle::ctrl_multiplier",
            self.ctrl_multiplier,
            default.ctrl_multiplier,
        );
    }
}

pub(crate) struct NumberOptions<'a, T> {
    pub(crate) range: RangeInclusive<T>,
    pub(crate) step: T,
    pub(crate) precision: Option<usize>,
    pub(crate) formatter: Option<Box<dyn Fn(T) -> String + 'a>>,
    pub(crate) prefix: String,
    pub(crate) suffix: String,
    pub(crate) enabled: bool,
    pub(crate) status: super::SemanticStatus,
    pub(crate) rounding: Option<CornerRadius>,
    pub(crate) hover_style: Option<super::HoverStyle>,
    pub(crate) id: Option<Id>,
    pub(crate) source: &'static Location<'static>,
    pub(crate) style: Option<NumberStyle>,
    pub(crate) width: Option<f32>,
    pub(crate) sensitivity: Option<f64>,
}
impl<T: Numeric> NumberOptions<'_, T> {
    pub(crate) fn new(source: &'static Location<'static>) -> Self {
        Self {
            range: T::MIN..=T::MAX,
            step: T::ONE,
            precision: None,
            formatter: None,
            prefix: String::new(),
            suffix: String::new(),
            enabled: true,
            status: Default::default(),
            rounding: None,
            hover_style: None,
            id: None,
            source,
            style: None,
            width: None,
            sensitivity: None,
        }
    }
    fn display(&self, value: T) -> String {
        self.formatter
            .as_ref()
            .map_or_else(|| value.formatted(self.precision), |f| f(value))
    }
}

macro_rules! number_builders {
    () => {
        pub fn id_source(mut self, source: impl Hash) -> Self {
            self.options.id = Some(Id::new(source));
            self
        }
        /// Commits and gestures clamp to the bounds; idle values are untouched. Descending
        /// bounds are swapped and non-finite ones are ignored.
        #[track_caller]
        pub fn range(mut self, range: RangeInclusive<T>) -> Self {
            let (start, end) = (*range.start(), *range.end());
            if !start.finite() || !end.finite() {
                crate::context::invalid_value(
                    "range",
                    format!("expected finite bounds, got {start}..={end}; ignored"),
                );
            } else if start > end {
                crate::context::invalid_value(
                    "range",
                    format!("expected ascending bounds, got {start}..={end}; ends swapped"),
                );
                self.options.range = end..=start;
            } else {
                self.options.range = range;
            }
            self
        }
        /// Positive step in the value's own type; integers default to one. Typed commits
        /// are not snapped. Anything not positive is ignored.
        #[track_caller]
        pub fn step(mut self, step: T) -> Self {
            if step.positive() {
                self.options.step = step;
            } else {
                crate::context::invalid_value(
                    "step",
                    format!("expected a finite step > 0, got {step}; ignored"),
                );
            }
            self
        }
        /// Display precision for floats, without rounding the stored number; at most 16.
        /// Integers never show decimals.
        pub fn precision(mut self, precision: usize) -> Self {
            self.options.precision = Some(precision.min(16));
            self
        }
        /// Display-only formatter. Editing always uses the exact, parseable number.
        pub fn formatter(mut self, formatter: impl Fn(T) -> String + 'a) -> Self {
            self.options.formatter = Some(Box::new(formatter));
            self
        }
        pub fn prefix(mut self, text: impl Into<String>) -> Self {
            self.options.prefix = text.into();
            self
        }
        pub fn suffix(mut self, text: impl Into<String>) -> Self {
            self.options.suffix = text.into();
            self
        }
        pub fn enabled(mut self, enabled: bool) -> Self {
            self.options.enabled = enabled;
            self
        }
        #[deprecated(note = "use `.enabled(!disabled)`; `enabled` is the one way to set a control's availability")]
        pub fn disabled(self, disabled: bool) -> Self {
            self.enabled(!disabled)
        }
        /// Validation state: the border and a soft ring take the status color. Fields inherit
        /// the status of an enclosing [`super::Field`] unless this is set.
        pub fn status(mut self, status: crate::SemanticStatus) -> Self {
            self.options.status = status;
            self
        }
        pub fn corner_radius(mut self, radius: impl Into<crate::CornerRadius>) -> Self {
            self.options.rounding = Some(radius.into());
            self
        }
        pub fn hover_style(mut self, style: crate::HoverStyle) -> Self {
            self.options.hover_style = Some(style);
            self
        }
        #[track_caller]
        pub fn width(mut self, width: f32) -> Self {
            self.options.width =
                crate::components::sanitize::positive("width", width).or(self.options.width);
            self
        }
        pub fn style(mut self, style: NumberStyle) -> Self {
            self.options.style = Some(style);
            self
        }
        /// Steps per logical pixel. The typed step determines the units.
        #[track_caller]
        pub fn sensitivity(mut self, sensitivity: f64) -> Self {
            if sensitivity.is_finite() && sensitivity > 0.0 {
                self.options.sensitivity = Some(sensitivity);
            } else {
                crate::context::invalid_value(
                    "sensitivity",
                    format!("expected a finite value > 0, got {sensitivity}; ignored"),
                );
            }
            self
        }
    };
}
pub(crate) use number_builders;

/// Numeric text entry with a draft independent of the confirmed value.
/// Enter commits valid input; Escape restores the focus-session value.
/// Focus loss commits valid input and discards invalid input. Disabled discards drafts.
/// Up/Down adjust the confirmed value; Shift is precise and Ctrl accelerated.
pub struct NumberInput<'a, T: Numeric> {
    value: &'a mut T,
    options: NumberOptions<'a, T>,
}
impl<'a, T: Numeric> NumberInput<'a, T> {
    #[track_caller]
    pub fn new(value: &'a mut T) -> Self {
        Self {
            value,
            options: NumberOptions::new(Location::caller()),
        }
    }
    number_builders!();
}
impl<T: Numeric> Widget for NumberInput<'_, T> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        render::show(ui, self.value, self.options, false)
    }
}
impl Ui<'_> {
    #[track_caller]
    pub fn number_input<T: Numeric>(&mut self, value: &mut T) -> Response {
        self.add(NumberInput::new(value))
    }
}

pub(crate) fn show_drag<T: Numeric>(
    ui: &mut Ui<'_>,
    value: &mut T,
    options: NumberOptions<'_, T>,
) -> Response {
    render::show(ui, value, options, true)
}

#[derive(Default)]
pub(crate) struct NumberState {
    buffer: String,
    snapshot: String,
    last_value: String,
    editing: bool,
    origin: String,
    units: f64,
    last_step: String,
    pointer: Option<crate::Vec2>,
    distance: f32,
    focused: bool,
    invalid: bool,
    pub(crate) last_frame: u64,
}
