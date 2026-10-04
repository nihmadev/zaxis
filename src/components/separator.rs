use crate::{context::Paint, Border, Color, CornerRadius, Layout, Rect, Shape, Vec2};

use super::{Response, Ui, Widget};

/// A non-interactive dividing line, measured in logical pixels.
/// Horizontal by default, filling the available width with the style's border color.
/// Normal layout spacing still applies in addition to [`Self::spacing`].
pub struct Separator {
    direction: Layout,
    length: Option<f32>,
    thickness: Option<f32>,
    color: Option<Color>,
    style: crate::SeparatorStyle,
    spacing: Option<f32>,
    inset: Option<f32>,
}

impl Default for Separator {
    fn default() -> Self {
        Self::new()
    }
}

impl Separator {
    pub fn new() -> Self {
        Self {
            direction: Layout::Horizontal,
            length: None,
            thickness: None,
            color: None,
            style: Default::default(),
            spacing: None,
            inset: None,
        }
    }

    /// A vertical line with an explicit height, suitable for horizontal rows.
    /// Content extending past the UI's bounds is clipped, like other widgets.
    pub fn vertical(length: f32) -> Self {
        Self {
            direction: Layout::Vertical,
            ..Self::new().length(length)
        }
    }

    /// Preferred width (horizontal) or height (vertical). Width is limited by the UI.
    pub fn style(mut self, style: crate::SeparatorStyle) -> Self {
        self.style = style;
        self
    }
    #[track_caller]
    pub fn length(mut self, length: f32) -> Self {
        self.length = super::sanitize::non_negative("Separator::length", length).or(self.length);
        self
    }

    #[track_caller]
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness =
            super::sanitize::non_negative("Separator::thickness", thickness).or(self.thickness);
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Extra empty space on both sides perpendicular to the line.
    #[track_caller]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing =
            super::sanitize::non_negative("Separator::spacing", spacing).or(self.spacing);
        self
    }

    /// Shorten both ends without changing the allocated layout size.
    #[track_caller]
    pub fn inset(mut self, inset: f32) -> Self {
        self.inset = super::sanitize::non_negative("Separator::inset", inset).or(self.inset);
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = ui.next_id("separator");
        let mut style = ui.style().separator;
        style.merge(self.style);
        let thickness = self.thickness.or(style.thickness).unwrap_or(1.0).max(0.0);
        let spacing = self.spacing.or(style.spacing).unwrap_or(0.0).max(0.0);
        let inset = self.inset.or(style.inset).unwrap_or(0.0).max(0.0);
        let width = ui.available_width();
        let size = match self.direction {
            Layout::Horizontal => Vec2::new(
                self.length.unwrap_or(width).min(width),
                thickness + spacing * 2.0,
            ),
            Layout::Vertical => Vec2::new(
                (thickness + spacing * 2.0).min(width),
                self.length.unwrap_or(0.0),
            ),
        };
        let rect = ui.allocate_space(size);
        let (offset, line_size) = match self.direction {
            Layout::Horizontal => {
                let inset = inset.min(size.x * 0.5);
                (
                    Vec2::new(inset, spacing),
                    Vec2::new(size.x - inset * 2.0, thickness),
                )
            }
            Layout::Vertical => {
                let inset = inset.min(size.y * 0.5);
                let thickness = thickness.min(size.x);
                (
                    Vec2::new((size.x - thickness) * 0.5, inset),
                    Vec2::new(thickness, size.y - inset * 2.0),
                )
            }
        };
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![Paint::Shape(Shape::Rect {
                rect: Rect::from_min_size(rect.min + offset, line_size),
                fill: self
                    .color
                    .or(style.color)
                    .unwrap_or(ui.style().border.color),
                rounding: CornerRadius::ZERO,
                border: Border::NONE,
            })],
        );
        ui.response(id, rect, false)
    }
}

impl Ui<'_> {
    /// Add a horizontal separator using the available width and default styling.
    pub fn separator(&mut self) -> Response {
        self.add(Separator::new())
    }
}
