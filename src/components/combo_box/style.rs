use crate::{Border, Color, CornerRadius, Padding};
use std::time::Duration;

/// Compact dropdown proportions and palette, in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboBoxStyle {
    pub trigger: crate::ControlStyle,
    pub option: crate::ControlStyle,

    pub width: f32,
    pub trigger_height: f32,
    pub label_height: f32,
    pub label_gap: f32,
    pub row_height: f32,
    pub row_gap: f32,
    pub visible_rows: usize,
    pub popup_gap: f32,
    pub trigger_padding: Padding,
    pub popup_padding: Padding,
    pub rounding: CornerRadius,
    pub font_size: f32,
    pub label_font_size: f32,
    pub trigger_fill: Color,
    pub popup_fill: Color,
    pub text: Color,
    pub muted_text: Color,
    pub disabled_text: Color,
    pub check_color: Color,
    pub active_fill: Color,
    pub border: Border,
    pub hover_border: Border,
    pub popup_border: Border,
    pub animation_duration: Duration,
}
impl Default for ComboBoxStyle {
    fn default() -> Self {
        Self {
            trigger: Default::default(),
            option: Default::default(),

            width: 240.0,
            trigger_height: 22.0,
            label_height: 20.0,
            label_gap: 4.0,
            row_height: 20.0,
            row_gap: 2.0,
            visible_rows: 5,
            popup_gap: 4.0,
            trigger_padding: Padding::symmetric(8.0, 0.0),
            popup_padding: Padding::all(2.0),
            rounding: CornerRadius::all(4.0),
            font_size: 15.0,
            label_font_size: 16.0,
            trigger_fill: Color::gray(55),
            popup_fill: Color::gray(47),
            text: Color::gray(230),
            muted_text: Color::gray(159),
            disabled_text: Color::gray(100),
            check_color: Color::gray(230),
            active_fill: Color::gray(65),
            border: Border::new(1.0, Color::rgba(159, 159, 159, 38)),
            hover_border: Border::new(1.0, Color::rgba(159, 159, 159, 166)),
            popup_border: Border::new(1.0, Color::gray(78)),
            animation_duration: Duration::from_millis(160),
        }
    }
}
impl ComboBoxStyle {
    /// Out-of-range values are replaced as described in `components::sanitize`:
    /// lengths become non-negative, sizes that must be positive fall back to the default.
    #[track_caller]
    pub(super) fn normalize(&mut self) {
        use crate::components::sanitize::{length, positive};
        let default = Self::default();
        self.width = length("ComboBoxStyle::width", self.width);
        self.trigger_height = length("ComboBoxStyle::trigger_height", self.trigger_height);
        self.label_height = length("ComboBoxStyle::label_height", self.label_height);
        self.label_gap = length("ComboBoxStyle::label_gap", self.label_gap);
        self.row_gap = length("ComboBoxStyle::row_gap", self.row_gap);
        self.popup_gap = length("ComboBoxStyle::popup_gap", self.popup_gap);
        self.row_height =
            positive("ComboBoxStyle::row_height", self.row_height).unwrap_or(default.row_height);
        self.font_size =
            positive("ComboBoxStyle::font_size", self.font_size).unwrap_or(default.font_size);
        self.label_font_size = positive("ComboBoxStyle::label_font_size", self.label_font_size)
            .unwrap_or(default.label_font_size);
        self.visible_rows = self.visible_rows.max(1);
        for padding in [&mut self.trigger_padding, &mut self.popup_padding] {
            padding.left = length("ComboBoxStyle padding", padding.left);
            padding.right = length("ComboBoxStyle padding", padding.right);
            padding.top = length("ComboBoxStyle padding", padding.top);
            padding.bottom = length("ComboBoxStyle padding", padding.bottom);
        }
    }
}
