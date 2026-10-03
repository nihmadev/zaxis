use crate::{Border, Color, CornerRadius, Padding};

/// A neutral grey theme. Colors use sRGB; measurements use logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub accent: Color,
    pub on_accent: Color,
    pub disabled_text: Color,
    pub selected_fill: Color,
    pub selected_text: Color,
    pub success: Color,
    pub on_success: Color,
    pub warning: Color,
    pub on_warning: Color,
    pub error: Color,
    pub on_error: Color,
    pub control_height: f32,
    pub opacity: f32,
    pub elevation: crate::Shadow,
    pub typography: super::theme::Typography,
    pub button: super::theme::ButtonStyle,
    pub checkbox: super::theme::CheckboxStyle,
    pub slider: super::theme::SliderStyle,
    pub text_edit: super::theme::TextEditStyle,
    pub window: super::theme::WindowStyle,
    pub title_bar: super::theme::TitleBarStyle,
    pub popup: super::theme::PopupStyle,
    pub tooltip: super::TooltipStyle,
    pub color_picker: super::theme::ColorPickerStyle,
    pub text: super::theme::TextStyle,
    pub separator: super::theme::SeparatorStyle,
    pub loader: super::theme::LoaderStyle,
    pub progress: super::theme::ProgressStyle,

    pub split: super::SplitStyle,
    pub collapsing: super::CollapsingStyle,
    pub tree: super::TreeStyle,
    pub number: super::NumberStyle,
    pub grid: super::GridStyle,
    pub table: super::TableStyle,
    pub combo_box: super::ComboBoxStyle,
    pub scroll: super::ScrollStyle,
    pub motion: crate::MotionStyle,
    pub background: Color,
    pub window_fill: Color,
    pub title_fill: Color,
    pub text_color: Color,
    pub muted_text: Color,
    pub border: Border,
    pub button_fill: Color,
    pub button_hovered: Color,
    /// Hover preset inherited by controls. Per-widget overrides can disable it.
    pub hover_style: super::HoverStyle,
    pub button_pressed: Color,
    pub text_edit_fill: Color,
    pub text_edit_hovered: Color,
    pub text_edit_placeholder: Color,
    pub text_edit_selection: Color,
    pub text_edit_padding: Padding,
    pub text_edit_rounding: CornerRadius,
    pub text_edit_width: f32,
    pub text_edit_height: f32,
    pub text_edit_font_size: f32,
    pub text_edit_cursor_width: f32,
    /// Zero keeps the focused cursor continuously visible without scheduling timers.
    pub text_edit_blink_interval: std::time::Duration,
    pub focus_border: Border,
    pub rounding: CornerRadius,
    pub window_padding: Padding,
    pub button_padding: Padding,
    pub font_size: f32,
    pub title_height: f32,
    pub spacing: f32,
    /// Default backdrop sigma for windows and control backgrounds. Zero disables blur.
    pub blur_radius: f32,
    /// Fill opacity multiplier while backdrop blur is enabled.
    pub blur_opacity: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            accent: Color::gray(230),
            on_accent: Color::gray(32),
            disabled_text: Color::gray(159),
            selected_fill: Color::gray(54),
            selected_text: Color::gray(230),
            success: Color::rgb(91, 184, 121),
            on_success: Color::BLACK,
            warning: Color::rgb(230, 177, 70),
            on_warning: Color::BLACK,
            error: Color::rgb(220, 94, 94),
            on_error: Color::BLACK,
            control_height: 24.0,
            opacity: 1.0,
            elevation: crate::Shadow {
                color: Color::TRANSPARENT,
                ..crate::Shadow::default()
            },
            typography: super::theme::Typography::default(),
            button: Default::default(),
            checkbox: Default::default(),
            slider: Default::default(),
            text_edit: Default::default(),
            window: Default::default(),
            title_bar: Default::default(),
            popup: Default::default(),
            tooltip: Default::default(),
            color_picker: Default::default(),
            text: Default::default(),
            separator: Default::default(),
            loader: Default::default(),
            progress: Default::default(),
            split: super::SplitStyle::default(),
            collapsing: super::CollapsingStyle::default(),
            tree: super::TreeStyle::default(),
            number: super::NumberStyle::default(),
            grid: super::GridStyle::default(),
            table: super::TableStyle::default(),
            combo_box: super::ComboBoxStyle::default(),
            scroll: super::ScrollStyle::default(),
            motion: crate::MotionStyle::default(),
            background: Color::gray(32),
            window_fill: Color::gray(47),
            title_fill: Color::gray(57),
            text_color: Color::gray(230),
            muted_text: Color::gray(159),
            border: Border::new(1.0, Color::gray(78)),
            button_fill: Color::gray(66),
            button_hovered: Color::gray(80),
            hover_style: super::HoverStyle {
                fill: Some(super::HoverFill::Theme),
                ..super::HoverStyle::shadow(crate::Shadow::default())
            },
            button_pressed: Color::gray(54),
            text_edit_fill: Color::gray(55),
            text_edit_hovered: Color::gray(61),
            text_edit_placeholder: Color::gray(146),
            text_edit_selection: Color::rgba(105, 135, 175, 130),
            text_edit_padding: Padding::symmetric(10.0, 4.0),
            text_edit_rounding: CornerRadius::all(6.0),
            text_edit_width: 240.0,
            text_edit_height: 30.0,
            text_edit_font_size: 14.0,
            text_edit_cursor_width: 1.0,
            text_edit_blink_interval: std::time::Duration::from_millis(500),
            focus_border: Border::new(1.5, Color::gray(178)),
            rounding: CornerRadius::all(7.0),
            window_padding: Padding::all(18.0),
            button_padding: Padding::symmetric(16.0, 9.0),
            font_size: 16.0,
            title_height: 36.0,
            spacing: 12.0,
            blur_radius: 0.0,
            blur_opacity: 0.72,
        }
    }
}

impl Style {
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur_radius = super::blur::normalize_radius(radius);
        self
    }

    pub(crate) fn backdrop_fill(&self, mut color: Color, radius: f32) -> Color {
        if radius > 0.0 {
            // Smooth the opaque-to-glass boundary when sigma animates from zero.
            let mix = radius.min(1.0);
            let opacity = 1.0 + (self.blur_opacity.clamp(0.0, 1.0) - 1.0) * mix;
            color.0[3] = (f32::from(color.0[3]) * opacity).round() as u8;
        }
        color
    }
}
