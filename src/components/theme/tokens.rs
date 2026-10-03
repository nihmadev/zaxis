use crate::{Color, Padding, Shadow};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
}
impl Density {
    pub(crate) fn factor(self) -> f32 {
        if self == Self::Compact {
            0.8
        } else {
            1.0
        }
    }
}

/// Semantic opaque sRGB defaults. Alpha remains supported for custom palettes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub background: Color,
    pub surface: Color,
    pub surface_raised: Color,
    pub surface_control: Color,
    pub hover: Color,
    pub pressed: Color,
    pub foreground: Color,
    pub muted: Color,
    pub disabled: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub selected: Color,
    pub on_selected: Color,
    pub focus: Color,
    pub border: Color,
    pub success: Color,
    pub on_success: Color,
    pub warning: Color,
    pub on_warning: Color,
    pub error: Color,
    pub on_error: Color,
}
impl Palette {
    pub fn dark() -> Self {
        Self {
            background: Color::gray(32),
            surface: Color::gray(47),
            surface_raised: Color::gray(57),
            surface_control: Color::gray(66),
            hover: Color::gray(80),
            pressed: Color::gray(54),
            foreground: Color::gray(230),
            muted: Color::gray(198),
            disabled: Color::gray(145),
            accent: Color::gray(230),
            on_accent: Color::gray(32),
            selected: Color::gray(77),
            on_selected: Color::gray(230),
            focus: Color::gray(200),
            border: Color::gray(100),
            success: Color::rgb(91, 184, 121),
            on_success: Color::BLACK,
            warning: Color::rgb(230, 177, 70),
            on_warning: Color::BLACK,
            error: Color::rgb(220, 94, 94),
            on_error: Color::BLACK,
        }
    }
    pub fn light() -> Self {
        Self {
            background: Color::gray(235),
            surface: Color::gray(250),
            surface_raised: Color::gray(242),
            surface_control: Color::gray(230),
            hover: Color::gray(215),
            pressed: Color::gray(205),
            foreground: Color::gray(30),
            muted: Color::gray(80),
            disabled: Color::gray(110),
            accent: Color::rgb(35, 85, 155),
            on_accent: Color::WHITE,
            selected: Color::rgb(202, 220, 243),
            on_selected: Color::gray(30),
            focus: Color::rgb(25, 65, 125),
            border: Color::gray(135),
            success: Color::rgb(25, 110, 55),
            on_success: Color::WHITE,
            warning: Color::rgb(135, 85, 0),
            on_warning: Color::WHITE,
            error: Color::rgb(175, 35, 40),
            on_error: Color::WHITE,
        }
    }
    pub fn high_contrast() -> Self {
        Self {
            background: Color::BLACK,
            surface: Color::BLACK,
            surface_raised: Color::gray(15),
            surface_control: Color::gray(20),
            hover: Color::gray(40),
            pressed: Color::gray(30),
            foreground: Color::WHITE,
            muted: Color::gray(220),
            disabled: Color::gray(160),
            accent: Color::rgb(255, 220, 70),
            on_accent: Color::BLACK,
            selected: Color::rgb(35, 60, 100),
            on_selected: Color::WHITE,
            focus: Color::rgb(100, 220, 255),
            border: Color::gray(210),
            ..Self::dark()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    pub spacing: f32,
    pub padding: Padding,
    pub control_height: f32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub elevation: Shadow,
    pub blur: f32,
    pub opacity: f32,
    pub blur_opacity: f32,
}
impl Default for Metrics {
    fn default() -> Self {
        Self {
            spacing: 12.0,
            padding: Padding::all(18.0),
            control_height: 34.0,
            corner_radius: 7.0,
            border_width: 1.0,
            elevation: Shadow {
                color: Color::TRANSPARENT,
                ..Shadow::default()
            },
            blur: 0.0,
            opacity: 1.0,
            blur_opacity: 0.72,
        }
    }
}
/// Sizes only. Font family is chosen by Context::with_font; no weight manager.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub small: f32,
    pub body: f32,
    pub heading: f32,
    pub title: f32,
}
impl Default for Typography {
    fn default() -> Self {
        Self {
            small: 14.0,
            body: 16.0,
            heading: 20.0,
            title: 24.0,
        }
    }
}

/// WCAG contrast ratio for opaque colors. Composite translucent colors first.
pub fn contrast_ratio(a: Color, b: Color) -> f32 {
    let luma = |c: Color| {
        let c = c.linear();
        c[0] * 0.2126 + c[1] * 0.7152 + c[2] * 0.0722
    };
    let (a, b) = (luma(a), luma(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}
pub fn contrast_foreground(background: Color) -> Color {
    if contrast_ratio(background, Color::WHITE) > contrast_ratio(background, Color::BLACK) {
        Color::WHITE
    } else {
        Color::BLACK
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TypographyRole {
    Small,
    #[default]
    Body,
    Heading,
    Title,
}
