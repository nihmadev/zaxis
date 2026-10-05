use crate::{Border, Color, CornerRadius, FontWeight, Gradient, Padding, Shadow, Vec2};

/// Missing properties inherit; transparent/zero/NONE are ordinary explicit values.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceStyle {
    pub fill: Option<Gradient>,
    pub foreground: Option<Color>,
    pub border: Option<Border>,
    pub rounding: Option<CornerRadius>,
    pub shadow: Option<Shadow>,
    pub blur: Option<f32>,
    pub opacity: Option<f32>,
}
impl SurfaceStyle {
    pub fn fill(color: Color) -> Self {
        Self {
            fill: Some(Gradient::new(color, color)),
            ..Self::default()
        }
    }
    pub fn gradient(fill: Gradient) -> Self {
        Self {
            fill: Some(fill),
            ..Self::default()
        }
    }
    pub(crate) fn merge(&mut self, rhs: Self) {
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f; })* }; }
        set!(fill, foreground, border, rounding, shadow, blur, opacity);
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SemanticStatus {
    #[default]
    Normal,
    Success,
    Warning,
    Error,
}

/// Selection and focus are independent of pointer state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ControlState {
    pub enabled: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub selected: bool,
    pub focus: bool,
    pub status: SemanticStatus,
}
impl ControlState {
    pub fn from_response(r: super::super::Response, selected: bool) -> Self {
        Self {
            enabled: r.enabled,
            hovered: r.enabled && r.hovered,
            pressed: r.enabled && r.pressed,
            focus: r.enabled && r.focus_visible,
            selected,
            status: SemanticStatus::Normal,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ControlStyle {
    pub idle: SurfaceStyle,
    pub hover: SurfaceStyle,
    pub pressed: SurfaceStyle,
    pub disabled: SurfaceStyle,
    pub selected: SurfaceStyle,
    pub focus: SurfaceStyle,
    pub success: SurfaceStyle,
    pub warning: SurfaceStyle,
    pub error: SurfaceStyle,
}
impl ControlStyle {
    pub(crate) fn has_blur_override(self) -> bool {
        [
            self.idle,
            self.hover,
            self.pressed,
            self.disabled,
            self.selected,
            self.focus,
            self.success,
            self.warning,
            self.error,
        ]
        .iter()
        .any(|s| s.blur.is_some())
    }

    pub(crate) fn merge(&mut self, rhs: Self) {
        macro_rules! set { ($($f:ident),*) => { $(self.$f.merge(rhs.$f);)* }; }
        set!(idle, hover, pressed, disabled, selected, focus, success, warning, error);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ButtonStyle {
    pub surface: ControlStyle,
    pub padding: Option<Padding>,
    pub min_size: Option<Vec2>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CheckboxStyle {
    pub body: ControlStyle,
    pub indicator: ControlStyle,
    pub size: Option<f32>,
    pub gap: Option<f32>,
    pub indicator_width: Option<f32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwitchStyle {
    pub track: ControlStyle,
    pub thumb: ControlStyle,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub gap: Option<f32>,
    pub thumb_inset: Option<f32>,
}
/// Look of [`RadioGroup`](crate::RadioGroup): a thin ring (`body`), an inner `dot` and a soft
/// `halo` around the indicator. Unset surfaces fall back to the muted outline, the accent
/// dot and a translucent accent halo (selected, and while pressed); `body.hover` replaces the
/// darkened outline, `halo` the pressed glow. Metrics are logical pixels for the Medium size.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RadioStyle {
    pub body: ControlStyle,
    pub dot: ControlStyle,
    pub halo: ControlStyle,
    /// Diameter of the indicator.
    pub size: Option<f32>,
    /// Diameter of the inner dot.
    pub dot_size: Option<f32>,
    /// How far the halo reaches beyond the indicator.
    pub halo_size: Option<f32>,
    pub border_width: Option<f32>,
    /// Distance between the indicator and the label.
    pub gap: Option<f32>,
    pub row_gap: Option<f32>,
    pub column_gap: Option<f32>,
    /// Lower bound of a row's height; defaults to `Style::control_height`.
    pub row_height: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    /// Spring of the dot growing in; a value below critical damping gives the overshoot.
    pub dot_spring: Option<crate::SpringOptions>,
}
/// Look of [`SegmentedControl`](crate::SegmentedControl): a plate (`track`) holding a row of
/// segments, with a raised `thumb` under the selected one. `segment` colors the labels and
/// icons per state (`foreground`). A transparent track with a bordered, shadowless thumb gives
/// an outlined variant; a transparent thumb with a bottom-heavy border, an underlined one.
/// The thumb radius is the track radius minus `padding` unless `thumb.rounding` is set.
/// [`Style::segmented_outlined`](crate::Style) holds the same fields for the outlined
/// variant, where `thumb.fill` is the tonal fill of the selected segment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SegmentedStyle {
    pub track: SurfaceStyle,
    pub thumb: SurfaceStyle,
    pub segment: ControlStyle,
    /// Inset between the plate and the segments, in logical pixels.
    pub padding: Option<f32>,
    /// Distance between segments.
    pub gap: Option<f32>,
    pub icon_size: Option<f32>,
    /// Distance between an icon and its label.
    pub icon_gap: Option<f32>,
    /// Horizontal padding inside one segment.
    pub segment_padding: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    /// Line between neighbouring segments, drawn by the outlined variant.
    pub divider: Option<Border>,
    /// Draw a check mark on the selected segment (in place of its icon); outlined variant.
    pub mark: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderStyle {
    pub track: ControlStyle,
    pub fill: ControlStyle,
    pub thumb: ControlStyle,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub track_height: Option<f32>,
    pub thumb_radius: Option<f32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextEditStyle {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
    /// The family of field text; `None` is proportional. `TextEdit::monospace` overrides it.
    pub font_family: Option<crate::TextFamily>,
    pub padding: Option<Padding>,
    pub rounding: Option<CornerRadius>,
    pub placeholder: Option<Color>,
    pub selection: Option<Color>,
    pub cursor_width: Option<f32>,
    pub blink_interval: Option<std::time::Duration>,
    pub surface: ControlStyle,
    pub caret: Option<Color>,
    pub selection_foreground: Option<Color>,
    /// Padding of a multi-line field (`TextEdit::multiline`); single-line fields use `padding`.
    pub area_padding: Option<Padding>,
    /// Minimum height of a multi-line field with default sizing.
    pub area_min_height: Option<f32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowStyle {
    pub padding: Option<Padding>,
    pub title_height: Option<f32>,
    pub body: SurfaceStyle,
    pub title: SurfaceStyle,
    pub title_font_size: Option<f32>,
    pub title_font_weight: Option<FontWeight>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TitleBarStyle {
    pub surface: SurfaceStyle,
    pub controls: ControlStyle,
    pub height: Option<f32>,
    pub button_width: Option<f32>,
    pub font_size: Option<f32>,
    pub font_weight: Option<FontWeight>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PopupStyle {
    pub surface: SurfaceStyle,
    pub padding: Option<Padding>,
    pub gap: Option<f32>,
    pub spacing: Option<f32>,
}
/// Framed content container: fill, hairline border, radius and optional shadow.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CardStyle {
    pub surface: SurfaceStyle,
    pub padding: Option<Padding>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ColorPickerStyle {
    pub body: ControlStyle,
    pub field: ControlStyle,
    pub width: Option<f32>,
    pub row_height: Option<f32>,
    pub palette_height: Option<f32>,
    pub hue_height: Option<f32>,
    pub gap: Option<f32>,
    pub field_gap: Option<f32>,
    pub rounding: Option<CornerRadius>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextStyle {
    pub color: Option<Color>,
    pub muted: Option<Color>,
    pub size: Option<f32>,
    pub weight: Option<FontWeight>,
    /// The font family of text; `None` follows the typography role.
    pub family: Option<crate::TextFamily>,
    /// Tabular figures for proportional text; `None` leaves them off.
    pub tabular_numbers: Option<bool>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SeparatorStyle {
    pub color: Option<Color>,
    pub thickness: Option<f32>,
    pub spacing: Option<f32>,
    pub inset: Option<f32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LoaderStyle {
    pub color: Option<Color>,
    pub size: Option<f32>,
    pub stroke: Option<f32>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProgressStyle {
    pub track: SurfaceStyle,
    pub fill: SurfaceStyle,
    pub size: Option<Vec2>,
}

macro_rules! merge_fields {
    ($ty:ty; $($simple:ident),*; $($nested:ident),*) => {
        impl $ty {
            pub(crate) fn merge(&mut self, rhs: Self) {
                $(if rhs.$simple.is_some() { self.$simple = rhs.$simple; })*
                $(self.$nested.merge(rhs.$nested);)*
            }
        }
    };
}
merge_fields!(ButtonStyle; padding,min_size,font_size,font_weight; surface);
merge_fields!(CheckboxStyle; size,gap,indicator_width; body,indicator);
merge_fields!(SwitchStyle; width,height,gap,thumb_inset; track,thumb);
merge_fields!(SegmentedStyle; padding,gap,icon_size,icon_gap,segment_padding,font_size,font_weight,divider,mark; track,thumb,segment);
merge_fields!(RadioStyle; size,dot_size,halo_size,border_width,gap,row_gap,column_gap,row_height,font_size,font_weight,dot_spring; body,dot,halo);
merge_fields!(SliderStyle; width,height,track_height,thumb_radius; track,fill,thumb);
merge_fields!(TextEditStyle; width,height,font_size,font_weight,font_family,padding,rounding,placeholder,selection,cursor_width,blink_interval,caret,selection_foreground,area_padding,area_min_height; surface);
merge_fields!(WindowStyle; padding,title_height,title_font_size,title_font_weight; body,title);
merge_fields!(TitleBarStyle; height,button_width,font_size,font_weight; surface,controls);
merge_fields!(PopupStyle; padding,gap,spacing; surface);
merge_fields!(CardStyle; padding; surface);
merge_fields!(ColorPickerStyle; width,row_height,palette_height,hue_height,gap,field_gap,rounding; body,field);
merge_fields!(TextStyle; color,muted,size,weight,family,tabular_numbers; );
merge_fields!(SeparatorStyle; color,thickness,spacing,inset; );
merge_fields!(LoaderStyle; color,size,stroke; );
merge_fields!(ProgressStyle; size; track,fill);
