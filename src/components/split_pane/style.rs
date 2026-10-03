use crate::{Border, Color, CornerRadius, Padding, Shadow};

/// Shared or per-panel surface. Rounding does not depend on a visible handle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitSurface {
    pub fill: Color,
    pub rounding: CornerRadius,
    pub border: Border,
    pub padding: Padding,
    pub shadow: Option<Shadow>,
    pub blur: f32,
}
impl Default for SplitSurface {
    fn default() -> Self {
        Self {
            fill: Color::TRANSPARENT,
            rounding: CornerRadius::ZERO,
            border: Border::NONE,
            padding: Padding::all(0.0),
            shadow: None,
            blur: 0.0,
        }
    }
}
impl SplitSurface {
    pub fn fill(mut self, fill: Color) -> Self {
        self.fill = fill;
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = radius.into();
        self
    }
    pub fn border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
    pub fn shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }
    pub fn blur(mut self, blur: f32) -> Self {
        self.blur = blur;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SplitHandle {
    #[default]
    Line,
    Grip,
    /// Shows a central indicator only on hover, focus or capture.
    Invisible,
    /// Hit zone lies wholly inside the trailing edge of the preceding panel.
    /// Boundary takes priority over child controls only in this zone.
    PanelEdge,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitHandleStyle {
    pub kind: SplitHandle,
    pub hit_width: f32,
    pub thickness: f32,
    pub length: f32,
    pub rounding: CornerRadius,
    pub inset: f32,
    pub idle: Color,
    pub hover: Color,
    pub pressed: Color,
    pub focus: Color,
    pub capture_indicator: bool,
}
impl Default for SplitHandleStyle {
    fn default() -> Self {
        Self {
            kind: SplitHandle::Line,
            hit_width: 10.0,
            thickness: 1.0,
            length: 32.0,
            rounding: CornerRadius::all(3.0),
            inset: 8.0,
            idle: Color::gray(78),
            hover: Color::gray(155),
            pressed: Color::gray(205),
            focus: Color::rgb(130, 173, 226),
            capture_indicator: false,
        }
    }
}
impl SplitHandleStyle {
    pub fn kind(mut self, kind: SplitHandle) -> Self {
        self.kind = kind;
        self
    }
    pub fn hit_width(mut self, width: f32) -> Self {
        self.hit_width = width;
        self
    }
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }
    pub fn length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = radius.into();
        self
    }
    pub fn inset(mut self, inset: f32) -> Self {
        self.inset = inset;
        self
    }
    pub fn colors(mut self, idle: Color, hover: Color, pressed: Color, focus: Color) -> Self {
        self.idle = idle;
        self.hover = hover;
        self.pressed = pressed;
        self.focus = focus;
        self
    }
    pub fn capture_indicator(mut self, enabled: bool) -> Self {
        self.capture_indicator = enabled;
        self
    }
}

/// `Style::split` < SplitPane::style < individual builders < per-panel overrides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitStyle {
    pub container: SplitSurface,
    pub panel: SplitSurface,
    pub gap: f32,
    pub handle: SplitHandleStyle,
    pub keyboard_step: f32,
    pub keyboard_large_step: f32,
    pub spacing: f32,
}
impl Default for SplitStyle {
    fn default() -> Self {
        Self {
            container: SplitSurface::default(),
            panel: SplitSurface::default(),
            gap: 6.0,
            handle: SplitHandleStyle::default(),
            keyboard_step: 4.0,
            keyboard_large_step: 24.0,
            spacing: 8.0,
        }
    }
}
impl SplitStyle {
    pub fn container(mut self, surface: SplitSurface) -> Self {
        self.container = surface;
        self
    }
    pub fn panel(mut self, surface: SplitSurface) -> Self {
        self.panel = surface;
        self
    }
    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
    pub fn handle(mut self, handle: SplitHandleStyle) -> Self {
        self.handle = handle;
        self
    }
}
