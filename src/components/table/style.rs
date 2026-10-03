use crate::{Border, Color, CornerRadius, Padding, ScrollStyle};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TableStyle {
    pub row: crate::ControlStyle,
    pub header: crate::ControlStyle,

    pub rounding: CornerRadius,
    pub border: Border,
    pub padding: Padding,
    pub cell_padding: Padding,
    pub header_height: f32,
    pub row_min_height: f32,
    pub component_spacing: f32,
    pub font_size: f32,
    pub fill: Color,
    pub header_fill: Color,
    pub text_color: Color,
    pub alternate_fill: Color,
    pub selected_fill: Color,
    pub hovered_fill: Color,
    pub separator_color: Color,
    pub separator_width: f32,
    pub resize_handle_width: f32,
    pub striped: bool,
    pub separators: bool,
    pub scroll: ScrollStyle,
}
impl Default for TableStyle {
    fn default() -> Self {
        Self {
            row: Default::default(),
            header: Default::default(),

            rounding: CornerRadius::all(8.0),
            border: Border::new(1.0, Color::gray(72)),
            padding: Padding::all(8.0),
            cell_padding: Padding::symmetric(8.0, 4.0),
            header_height: 32.0,
            row_min_height: 30.0,
            component_spacing: 6.0,
            font_size: 14.0,
            fill: Color::gray(51),
            header_fill: Color::gray(57),
            text_color: Color::gray(230),
            alternate_fill: Color::gray(55),
            selected_fill: Color::gray(77),
            hovered_fill: Color::gray(62),
            separator_color: Color::gray(72),
            separator_width: 1.0,
            resize_handle_width: 8.0,
            striped: true,
            separators: false,
            scroll: ScrollStyle {
                padding: Padding::all(0.0),
                spacing: 0.0,
                ..ScrollStyle::default()
            },
        }
    }
}
