use crate::{Border, Color, CornerRadius, Padding};
use std::time::Duration;

/// Compact dropdown proportions and palette, in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboBoxStyle {
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
    pub(super) fn validate(&self) {
        assert!([
            self.width,
            self.trigger_height,
            self.label_height,
            self.label_gap,
            self.row_gap,
            self.popup_gap
        ]
        .iter()
        .all(|n| n.is_finite() && *n >= 0.0));
        assert!(self.row_height.is_finite() && self.row_height > 0.0);
        assert!(self.font_size.is_finite() && self.font_size > 0.0);
        assert!(self.label_font_size.is_finite() && self.label_font_size > 0.0);
        assert!(self.visible_rows > 0);
        for padding in [self.trigger_padding, self.popup_padding] {
            assert!([padding.left, padding.right, padding.top, padding.bottom]
                .iter()
                .all(|n| n.is_finite() && *n >= 0.0));
        }
    }
}
