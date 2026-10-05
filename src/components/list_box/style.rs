use crate::{Border, Color, CornerRadius, FontWeight, Padding, ScrollStyle};
use std::time::Duration;

/// Row density: scales the row height and padding of a [`ListBox`](crate::ListBox).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListDensity {
    Compact,
    #[default]
    Normal,
    Comfortable,
}
impl ListDensity {
    pub(super) fn factor(self) -> f32 {
        match self {
            Self::Compact => 0.8,
            Self::Normal => 1.0,
            Self::Comfortable => 1.3,
        }
    }
}

/// Look of a [`ListBox`](crate::ListBox). Every field is optional; unset ones follow the
/// current [`Style`](crate::Style) (`selected_fill`, `button_hovered`, `focus_border`, ...).
/// Lengths are logical pixels.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ListBoxStyle {
    /// Height of an item row, including the gap around its highlight. Default: control height.
    pub row_height: Option<f32>,
    /// Height of a section header row. Default: the row height.
    pub header_height: Option<f32>,
    /// Height of a separator row. Default: 9.
    pub separator_height: Option<f32>,
    /// Space between the highlight and the row's edge, on every side.
    pub row_margin: Option<f32>,
    /// Space between the highlight and the content.
    pub row_padding: Option<Padding>,
    pub row_rounding: Option<CornerRadius>,
    /// Background of the whole list; transparent by default.
    pub surface: Option<Color>,
    pub border: Option<Border>,
    pub idle: Option<Color>,
    pub hover: Option<Color>,
    pub selected: Option<Color>,
    /// Selected row while the list does not have keyboard focus.
    pub selected_inactive: Option<Color>,
    /// Ring around the keyboard-active row while the list has focus.
    pub active_ring: Option<Border>,
    /// Fill of every other item row; transparent by default.
    pub stripe: Option<Color>,
    /// Line under each item row.
    pub divider: Option<Border>,
    pub text: Option<Color>,
    pub selected_text: Option<Color>,
    pub muted_text: Option<Color>,
    pub disabled_text: Option<Color>,
    pub header_fill: Option<Color>,
    pub header_text: Option<Color>,
    pub header_font_size: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    pub check_size: Option<f32>,
    pub scroll: Option<ScrollStyle>,
    /// How long typed characters keep extending the type-ahead search.
    pub type_ahead_timeout: Option<Duration>,
}
macro_rules! builders {
    ($($name:ident: $ty:ty),* $(,)?) => {
        impl ListBoxStyle { $(
            pub fn $name(mut self, value: $ty) -> Self { self.$name = Some(value); self }
        )* }
    };
}
builders!(
    row_height: f32, header_height: f32, separator_height: f32, row_margin: f32,
    row_padding: Padding, row_rounding: CornerRadius, surface: Color, border: Border,
    idle: Color, hover: Color, selected: Color, selected_inactive: Color,
    active_ring: Border, stripe: Color, divider: Border, text: Color, selected_text: Color,
    muted_text: Color, disabled_text: Color, header_fill: Color, header_text: Color,
    header_font_size: f32, font_size: f32, font_weight: FontWeight, check_size: f32,
    scroll: ScrollStyle, type_ahead_timeout: Duration,
);
impl ListBoxStyle {
    pub(crate) fn merge(&mut self, rhs: &Self) {
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f; })* }; }
        set!(
            row_height,
            header_height,
            separator_height,
            row_margin,
            row_padding,
            row_rounding,
            surface,
            border,
            idle,
            hover,
            selected,
            selected_inactive,
            active_ring,
            stripe,
            divider,
            text,
            selected_text,
            muted_text,
            disabled_text,
            header_fill,
            header_text,
            header_font_size,
            font_size,
            font_weight,
            check_size,
            scroll,
            type_ahead_timeout
        );
    }
}
