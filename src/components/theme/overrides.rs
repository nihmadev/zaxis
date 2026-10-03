use super::*;
use crate::{
    Border, Color, CornerRadius, Padding, ScrollStyle, SplitHandleStyle, SplitSurface, Vec2,
};
use std::time::Duration;

trait PatchValue: Clone {
    fn apply_to(&self, target: &mut Self) {
        *target = self.clone();
    }
}
macro_rules! values { ($($ty:ty),*)=> {$(impl PatchValue for $ty {})*}; }
values!(
    f32,
    f64,
    usize,
    bool,
    Duration,
    Vec2,
    Color,
    Border,
    CornerRadius,
    Padding,
    ScrollStyle,
    SplitSurface,
    SplitHandleStyle
);
impl PatchValue for ControlStyle {
    fn apply_to(&self, target: &mut Self) {
        target.merge(*self);
    }
}
impl PatchValue for SurfaceStyle {
    fn apply_to(&self, target: &mut Self) {
        target.merge(*self);
    }
}

macro_rules! patch {
    ($name:ident, $target:ty, {$($field:ident: $ty:ty),* $(,)?}) => {
        #[derive(Clone, Debug, Default, PartialEq)]
        pub struct $name { $(pub $field: Option<$ty>,)* }
        impl $name {
            pub fn apply(&self, style: &mut $target) {
                $(if let Some(value) = &self.$field { value.apply_to(&mut style.$field); })*
            }
        }
    };
}

patch!(NumberStyleOverride, crate::NumberStyle, {
    surface: crate::ControlStyle,

    width: f32,
    height: f32,
    padding: Padding,
    rounding: CornerRadius,
    font_size: f32,
    fill: Color,
    hovered: Color,
    invalid: Color,
    sensitivity: f64,
    shift_multiplier: f64,
    ctrl_multiplier: f64,
});

patch!(GridStyleOverride, crate::GridStyle, {
    surface: crate::SurfaceStyle,

    padding: Padding,
    cell_padding: Padding,
    spacing: Vec2,
    component_spacing: f32,
    row_min_height: f32,
    fill: Color,
    rounding: f32,
});

patch!(TableStyleOverride, crate::TableStyle, {
    row: crate::ControlStyle,
    header: crate::ControlStyle,

    rounding: CornerRadius,
    border: Border,
    padding: Padding,
    cell_padding: Padding,
    header_height: f32,
    row_min_height: f32,
    component_spacing: f32,
    font_size: f32,
    fill: Color,
    header_fill: Color,
    text_color: Color,
    alternate_fill: Color,
    selected_fill: Color,
    hovered_fill: Color,
    separator_color: Color,
    separator_width: f32,
    resize_handle_width: f32,
    striped: bool,
    separators: bool,
    scroll: ScrollStyle,
});

patch!(ComboBoxStyleOverride, crate::ComboBoxStyle, {
    trigger: crate::ControlStyle,
    option: crate::ControlStyle,

    width: f32,
    trigger_height: f32,
    label_height: f32,
    label_gap: f32,
    row_height: f32,
    row_gap: f32,
    visible_rows: usize,
    popup_gap: f32,
    trigger_padding: Padding,
    popup_padding: Padding,
    rounding: CornerRadius,
    font_size: f32,
    label_font_size: f32,
    trigger_fill: Color,
    popup_fill: Color,
    text: Color,
    muted_text: Color,
    disabled_text: Color,
    check_color: Color,
    active_fill: Color,
    border: Border,
    hover_border: Border,
    popup_border: Border,
    animation_duration: Duration,
});

patch!(ScrollStyleOverride, crate::ScrollStyle, {
    thumb: crate::ControlStyle,

    padding: Padding,
    spacing: f32,
    bar_width: f32,
    bar_margin: f32,
    min_thumb_length: f32,
    thumb_color: Color,
    thumb_hovered: Color,
    hint_size: f32,
    hint_color: Color,
});

patch!(SplitStyleOverride, crate::SplitStyle, {
    container: SplitSurface,
    panel: SplitSurface,
    gap: f32,
    handle: SplitHandleStyle,
    keyboard_step: f32,
    keyboard_large_step: f32,
    spacing: f32,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyleOverrides {
    pub accent: Option<Color>,
    pub on_accent: Option<Color>,
    pub disabled_text: Option<Color>,
    pub selected_fill: Option<Color>,
    pub selected_text: Option<Color>,
    pub success: Option<Color>,
    pub on_success: Option<Color>,
    pub warning: Option<Color>,
    pub on_warning: Option<Color>,
    pub error: Option<Color>,
    pub on_error: Option<Color>,
    pub control_height: Option<f32>,
    pub opacity: Option<f32>,
    pub elevation: Option<crate::Shadow>,
    pub typography: Option<Typography>,
    pub button: ButtonStyle,
    pub checkbox: CheckboxStyle,
    pub slider: SliderStyle,
    pub text_edit: TextEditStyle,
    pub window: WindowStyle,
    pub title_bar: TitleBarStyle,
    pub popup: PopupStyle,
    pub color_picker: ColorPickerStyle,
    pub text: TextStyle,
    pub separator: SeparatorStyle,
    pub loader: LoaderStyle,
    pub progress: ProgressStyle,
    pub split: SplitStyleOverride,
    pub collapsing: crate::CollapsingStyle,
    pub tree: crate::TreeStyle,
    pub number: NumberStyleOverride,
    pub grid: GridStyleOverride,
    pub table: TableStyleOverride,
    pub combo_box: ComboBoxStyleOverride,
    pub scroll: ScrollStyleOverride,
    pub motion: Option<crate::MotionStyle>,
    pub background: Option<Color>,
    pub window_fill: Option<Color>,
    pub title_fill: Option<Color>,
    pub text_color: Option<Color>,
    pub muted_text: Option<Color>,
    pub border: Option<Border>,
    pub button_fill: Option<Color>,
    pub button_hovered: Option<Color>,
    pub hover_style: Option<crate::HoverStyle>,
    pub button_pressed: Option<Color>,
    pub text_edit_fill: Option<Color>,
    pub text_edit_hovered: Option<Color>,
    pub text_edit_placeholder: Option<Color>,
    pub text_edit_selection: Option<Color>,
    pub text_edit_padding: Option<Padding>,
    pub text_edit_rounding: Option<CornerRadius>,
    pub text_edit_width: Option<f32>,
    pub text_edit_height: Option<f32>,
    pub text_edit_font_size: Option<f32>,
    pub text_edit_cursor_width: Option<f32>,
    pub text_edit_blink_interval: Option<std::time::Duration>,
    pub focus_border: Option<Border>,
    pub rounding: Option<CornerRadius>,
    pub window_padding: Option<Padding>,
    pub button_padding: Option<Padding>,
    pub font_size: Option<f32>,
    pub title_height: Option<f32>,
    pub spacing: Option<f32>,
    pub blur_radius: Option<f32>,
    pub blur_opacity: Option<f32>,
}
impl StyleOverrides {
    pub fn apply(&self, style: &mut crate::Style) {
        if let Some(value) = &self.accent {
            style.accent = value.clone();
        }
        if let Some(value) = &self.on_accent {
            style.on_accent = value.clone();
        }
        if let Some(value) = &self.disabled_text {
            style.disabled_text = value.clone();
        }
        if let Some(value) = &self.selected_fill {
            style.selected_fill = value.clone();
        }
        if let Some(value) = &self.selected_text {
            style.selected_text = value.clone();
        }
        if let Some(value) = &self.success {
            style.success = value.clone();
        }
        if let Some(value) = &self.on_success {
            style.on_success = value.clone();
        }
        if let Some(value) = &self.warning {
            style.warning = value.clone();
        }
        if let Some(value) = &self.on_warning {
            style.on_warning = value.clone();
        }
        if let Some(value) = &self.error {
            style.error = value.clone();
        }
        if let Some(value) = &self.on_error {
            style.on_error = value.clone();
        }
        if let Some(value) = &self.control_height {
            style.control_height = value.clone();
        }
        if let Some(value) = &self.opacity {
            style.opacity = value.clone();
        }
        if let Some(value) = &self.elevation {
            style.elevation = value.clone();
        }
        if let Some(value) = &self.typography {
            style.typography = value.clone();
        }
        style.button.merge(self.button);
        style.checkbox.merge(self.checkbox);
        style.slider.merge(self.slider);
        style.text_edit.merge(self.text_edit);
        style.window.merge(self.window);
        style.title_bar.merge(self.title_bar);
        style.popup.merge(self.popup);
        style.color_picker.merge(self.color_picker);
        style.text.merge(self.text);
        style.separator.merge(self.separator);
        style.loader.merge(self.loader);
        style.progress.merge(self.progress);
        self.split.apply(&mut style.split);
        style.collapsing.merge(&self.collapsing);
        style.tree.merge(&self.tree);
        self.number.apply(&mut style.number);
        self.grid.apply(&mut style.grid);
        self.table.apply(&mut style.table);
        self.combo_box.apply(&mut style.combo_box);
        self.scroll.apply(&mut style.scroll);
        if let Some(value) = &self.motion {
            style.motion = value.clone();
        }
        if let Some(value) = &self.background {
            style.background = value.clone();
        }
        if let Some(value) = &self.window_fill {
            style.window_fill = value.clone();
        }
        if let Some(value) = &self.title_fill {
            style.title_fill = value.clone();
        }
        if let Some(value) = &self.text_color {
            style.text_color = value.clone();
        }
        if let Some(value) = &self.muted_text {
            style.muted_text = value.clone();
        }
        if let Some(value) = &self.border {
            style.border = value.clone();
        }
        if let Some(value) = &self.button_fill {
            style.button_fill = value.clone();
        }
        if let Some(value) = &self.button_hovered {
            style.button_hovered = value.clone();
        }
        if let Some(value) = &self.hover_style {
            style.hover_style = value.clone();
        }
        if let Some(value) = &self.button_pressed {
            style.button_pressed = value.clone();
        }
        if let Some(value) = &self.text_edit_fill {
            style.text_edit_fill = value.clone();
        }
        if let Some(value) = &self.text_edit_hovered {
            style.text_edit_hovered = value.clone();
        }
        if let Some(value) = &self.text_edit_placeholder {
            style.text_edit_placeholder = value.clone();
        }
        if let Some(value) = &self.text_edit_selection {
            style.text_edit_selection = value.clone();
        }
        if let Some(value) = &self.text_edit_padding {
            style.text_edit_padding = value.clone();
        }
        if let Some(value) = &self.text_edit_rounding {
            style.text_edit_rounding = value.clone();
        }
        if let Some(value) = &self.text_edit_width {
            style.text_edit_width = value.clone();
        }
        if let Some(value) = &self.text_edit_height {
            style.text_edit_height = value.clone();
        }
        if let Some(value) = &self.text_edit_font_size {
            style.text_edit_font_size = value.clone();
        }
        if let Some(value) = &self.text_edit_cursor_width {
            style.text_edit_cursor_width = value.clone();
        }
        if let Some(value) = &self.text_edit_blink_interval {
            style.text_edit_blink_interval = value.clone();
        }
        if let Some(value) = &self.focus_border {
            style.focus_border = value.clone();
        }
        if let Some(value) = &self.rounding {
            style.rounding = value.clone();
        }
        if let Some(value) = &self.window_padding {
            style.window_padding = value.clone();
        }
        if let Some(value) = &self.button_padding {
            style.button_padding = value.clone();
        }
        if let Some(value) = &self.font_size {
            style.font_size = value.clone();
        }
        if let Some(value) = &self.title_height {
            style.title_height = value.clone();
        }
        if let Some(value) = &self.spacing {
            style.spacing = value.clone();
        }
        if let Some(value) = &self.blur_radius {
            style.blur_radius = value.clone();
        }
        if let Some(value) = &self.blur_opacity {
            style.blur_opacity = value.clone();
        }
    }
    pub fn resolve(&self, base: &crate::Style) -> crate::Style {
        let mut style = base.clone();
        self.apply(&mut style);
        style
    }
}
