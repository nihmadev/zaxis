use crate::{context::Paint, Border, Color, CornerRadius, Layout, Rect, Shape, Vec2};

use super::{Response, Ui, Widget};

/// A non-interactive dividing line, measured in logical pixels.
/// Horizontal by default, filling the available width with the style's border color.
/// Normal layout spacing still applies in addition to [`Self::spacing`].
pub struct Separator {
    direction: Layout,
    length: Option<f32>,
    thickness: f32,
    color: Option<Color>,
    spacing: f32,
    inset: f32,
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
            thickness: 1.0,
            color: None,
            spacing: 0.0,
            inset: 0.0,
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
    pub fn length(mut self, length: f32) -> Self {
        assert!(
            length.is_finite() && length >= 0.0,
            "separator length must be finite and non-negative"
        );
        self.length = Some(length);
        self
    }

    pub fn thickness(mut self, thickness: f32) -> Self {
        assert!(
            thickness.is_finite() && thickness > 0.0,
            "separator thickness must be finite and positive"
        );
        self.thickness = thickness;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    /// Extra empty space on both sides perpendicular to the line.
    pub fn spacing(mut self, spacing: f32) -> Self {
        assert!(
            spacing.is_finite() && spacing >= 0.0,
            "separator spacing must be finite and non-negative"
        );
        self.spacing = spacing;
        self
    }

    /// Shorten both ends without changing the allocated layout size.
    pub fn inset(mut self, inset: f32) -> Self {
        assert!(
            inset.is_finite() && inset >= 0.0,
            "separator inset must be finite and non-negative"
        );
        self.inset = inset;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = ui.next_id("separator");
        let width = ui.available_width();
        let size = match self.direction {
            Layout::Horizontal => Vec2::new(
                self.length.unwrap_or(width).min(width),
                self.thickness + self.spacing * 2.0,
            ),
            Layout::Vertical => Vec2::new(
                (self.thickness + self.spacing * 2.0).min(width),
                self.length.unwrap_or(0.0),
            ),
        };
        let rect = ui.allocate_space(size);
        let (offset, line_size) = match self.direction {
            Layout::Horizontal => {
                let inset = self.inset.min(size.x * 0.5);
                (
                    Vec2::new(inset, self.spacing),
                    Vec2::new(size.x - inset * 2.0, self.thickness),
                )
            }
            Layout::Vertical => {
                let inset = self.inset.min(size.y * 0.5);
                let thickness = self.thickness.min(size.x);
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
                fill: self.color.unwrap_or(ui.style().border.color),
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
