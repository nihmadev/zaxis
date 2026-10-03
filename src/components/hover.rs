use crate::{Border, Color, Gradient, Shadow};

use super::{Response, Ui, Widget, WidgetState};

/// Optional hover fill. Gradients follow the control's actual corner radii.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HoverFill {
    /// Use `Style::button_hovered`, preserving existing theme overrides.
    Theme,
    Solid(Color),
    Gradient(Gradient),
}

/// Reusable hover appearance. Unspecified properties keep the normal appearance.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HoverStyle {
    pub fill: Option<HoverFill>,
    pub shadow: Option<Shadow>,
    pub border: Option<Border>,
    pub text_color: Option<Color>,
}

impl HoverStyle {
    pub const NONE: Self = Self {
        fill: None,
        shadow: None,
        border: None,
        text_color: None,
    };

    pub const fn fill(color: Color) -> Self {
        Self {
            fill: Some(HoverFill::Solid(color)),
            ..Self::NONE
        }
    }

    pub const fn gradient(gradient: Gradient) -> Self {
        Self {
            fill: Some(HoverFill::Gradient(gradient)),
            ..Self::NONE
        }
    }

    pub const fn shadow(shadow: Shadow) -> Self {
        Self {
            shadow: Some(shadow),
            ..Self::NONE
        }
    }

    pub const fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub const fn with_border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    pub const fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
}

/// Apply a hover preset to any built-in control, or paint a custom hover overlay.
/// The wrapped widget keeps its ID, layout, hit testing and keyboard behavior.
pub struct Hover<W, F = fn(&mut Ui<'_>, Response)> {
    widget: W,
    style: Option<HoverStyle>,
    paint: F,
}

impl<W> Hover<W> {
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            style: None,
            paint: |_, _| {},
        }
    }
}

impl<W, F> Hover<W, F> {
    pub fn style(mut self, style: HoverStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Draw custom foreground geometry while hovered. Pressed/disabled controls skip it.
    pub fn on_hover<G: FnOnce(&mut Ui<'_>, Response)>(self, paint: G) -> Hover<W, G> {
        Hover {
            widget: self.widget,
            style: self.style,
            paint,
        }
    }
}

impl<W: Widget, F: FnOnce(&mut Ui<'_>, Response)> Widget for Hover<W, F> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let previous = ui.hover_style;
        if let Some(style) = self.style {
            ui.hover_style = Some(style);
        }
        let response = ui.add(self.widget);
        ui.hover_style = previous;
        if response.state() == WidgetState::Hovered {
            ui.push_id(("hover-overlay", response.id), |ui| {
                (self.paint)(ui, response)
            });
        }
        response
    }
}
