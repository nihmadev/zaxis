use crate::{Color, FontWeight, Padding, Shadow};

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
    /// Neutral gray scale with a single blue accent. Steps follow the usual
    /// 12-step UI scale: app bg, panel, control, hover, active, border.
    pub fn dark() -> Self {
        Self {
            background: Color::gray(17),
            surface: Color::gray(25),
            surface_raised: Color::gray(34),
            surface_control: Color::gray(34),
            hover: Color::gray(42),
            pressed: Color::gray(49),
            foreground: Color::gray(238),
            muted: Color::gray(180),
            disabled: Color::gray(110),
            accent: Color::rgb(10, 110, 235),
            on_accent: Color::WHITE,
            selected: Color::rgb(13, 40, 71),
            on_selected: Color::gray(238),
            focus: Color::rgb(59, 158, 255),
            border: Color::gray(58),
            success: Color::rgb(48, 164, 108),
            on_success: Color::BLACK,
            warning: Color::rgb(255, 197, 61),
            on_warning: Color::BLACK,
            error: Color::rgb(229, 72, 77),
            on_error: Color::BLACK,
        }
    }
    pub fn light() -> Self {
        Self {
            background: Color::gray(249),
            surface: Color::WHITE,
            surface_raised: Color::gray(252),
            surface_control: Color::gray(240),
            hover: Color::gray(232),
            pressed: Color::gray(224),
            foreground: Color::gray(32),
            muted: Color::gray(96),
            disabled: Color::gray(160),
            accent: Color::rgb(0, 102, 214),
            on_accent: Color::WHITE,
            selected: Color::rgb(230, 244, 254),
            on_selected: Color::gray(32),
            focus: Color::rgb(0, 102, 214),
            border: Color::gray(217),
            success: Color::rgb(21, 128, 61),
            on_success: Color::WHITE,
            warning: Color::rgb(161, 98, 7),
            on_warning: Color::WHITE,
            error: Color::rgb(200, 40, 48),
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
            spacing: 8.0,
            padding: Padding::all(16.0),
            control_height: 32.0,
            corner_radius: 5.0,
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
/// Sizes and weights per role. The family is chosen by `Context::with_fonts`
/// or `RunOptions::with_font_family`; the `code` role uses the monospace family
/// (`RunOptions::with_monospace_family`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub small: f32,
    pub body: f32,
    pub heading: f32,
    pub title: f32,
    /// Size of [`TypographyRole::Code`], a little smaller than body: monospace glyphs are wide.
    pub code: f32,
    pub weights: TypographyWeights,
}
impl Default for Typography {
    fn default() -> Self {
        Self {
            small: 12.0,
            body: 14.0,
            heading: 18.0,
            title: 24.0,
            code: 13.0,
            weights: TypographyWeights::default(),
        }
    }
}

/// Font weight per typographic role. All Regular by default, so a theme looks
/// the same until a weight is set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypographyWeights {
    pub small: FontWeight,
    pub body: FontWeight,
    pub heading: FontWeight,
    pub title: FontWeight,
    pub code: FontWeight,
    /// Labels of controls such as buttons, tabs and menu items.
    pub control: FontWeight,
    /// The active tab or selected item. `None` follows `control`.
    pub selected: Option<FontWeight>,
}
impl Default for TypographyWeights {
    fn default() -> Self {
        Self {
            small: FontWeight::REGULAR,
            body: FontWeight::REGULAR,
            heading: FontWeight::REGULAR,
            title: FontWeight::REGULAR,
            code: FontWeight::REGULAR,
            control: FontWeight::REGULAR,
            selected: None,
        }
    }
}
impl TypographyWeights {
    pub fn role(&self, role: TypographyRole) -> FontWeight {
        match role {
            TypographyRole::Small => self.small,
            TypographyRole::Body => self.body,
            TypographyRole::Heading => self.heading,
            TypographyRole::Title => self.title,
            TypographyRole::Code => self.code,
        }
    }
    pub fn selected(&self) -> FontWeight {
        self.selected.unwrap_or(self.control)
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
    /// Source code, hashes and logs: sized by `Typography::code` and set in the
    /// monospace family unless the text overrides it.
    Code,
}

impl Typography {
    /// The size of `role`, with `body` for [`TypographyRole::Body`].
    pub fn size(&self, role: TypographyRole) -> f32 {
        match role {
            TypographyRole::Small => self.small,
            TypographyRole::Body => self.body,
            TypographyRole::Heading => self.heading,
            TypographyRole::Title => self.title,
            TypographyRole::Code => self.code,
        }
    }
}
