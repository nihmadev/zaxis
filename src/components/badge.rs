use std::hash::Hash;

use super::{font_size, visible_label, Response, Ui, Widget};
use crate::{context::Paint, Border, Color, CornerRadius, FontWeight, Id, Padding, Shape, Vec2};

/// A small passive label on a filled, optionally outlined chip: a key cap, a status pill,
/// a count. It sizes itself to its text, takes no input and reports no events.
///
/// ```no_run
/// # fn chips(ui: &mut zaxis::Ui<'_>) {
/// use zaxis::{Badge, Color};
/// ui.horizontal(|ui| {
///     ui.add(Badge::new("RShift").fill(Color::gray(40)).font_size(11.0));
///     ui.add(Badge::new("On").fill(Color::rgb(30, 80, 50)).text_color(Color::WHITE));
/// });
/// # }
/// ```
pub struct Badge {
    text: String,
    id: Option<Id>,
    fill: Option<Color>,
    border: Option<Border>,
    text_color: Option<Color>,
    rounding: Option<CornerRadius>,
    padding: Padding,
    height: f32,
    font_size: Option<f32>,
    weight: Option<FontWeight>,
}

impl Badge {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            id: None,
            fill: None,
            border: None,
            text_color: None,
            rounding: None,
            padding: Padding::symmetric(6.0, 0.0),
            height: 18.0,
            font_size: None,
            weight: None,
        }
    }

    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    /// Chip color; `Style::button_fill`.
    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }
    /// Outline; none by default.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }
    /// Text color; `Style::text_color`.
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
    /// Corner radius; 4.
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
    }
    /// Space between the text and the chip's edges; 6 horizontally, none vertically.
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
    /// Minimum chip height; 18.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height.max(1.0);
        self
    }
    /// Font size; `Style::font_size`.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }
    /// Font weight; the control weight of the typography.
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }
}

impl Widget for Badge {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let style = ui.style().clone();
        let size = font_size(self.font_size.unwrap_or(style.font_size));
        let weight = self.weight.unwrap_or(style.typography.weights.control);
        let label = visible_label(&self.text);
        let text = ui.context.measure_text(label, size, weight, f32::INFINITY);
        let chip = Vec2::new(
            text.x + self.padding.size().x,
            self.height.max(text.y + self.padding.size().y),
        );
        let rect = ui.allocate_space(chip);
        let id = ui
            .scope
            .with(("badge", self.id.unwrap_or_else(|| Id::new(&self.text))));
        let response = ui.response(id, rect, false);
        let optical = ui.context.centered_line_offset(label, size, weight);
        let position = Vec2::new(
            rect.min.x + self.padding.left,
            rect.center().y - text.y * 0.5 + optical,
        );
        // A chip without text is a dot: decoration, nothing to announce.
        if ui.context.a11y_on() && !label.is_empty() {
            let layout = ui.context.paragraph_layout(
                label,
                size,
                weight,
                f32::INFINITY,
                crate::text::DEFAULT_TAB,
            );
            let lines = ui.context.a11y_text(label, &layout);
            ui.a11y(id, rect, crate::AccessRole::Label, |node| {
                node.value(label)
                    .text(lines, None)
                    .text_origin(position - rect.min);
            });
        }
        let rounding = self.rounding.unwrap_or(CornerRadius::all(4.0));
        let mut shape =
            Shape::rect(rect, self.fill.unwrap_or(style.button_fill)).corner_radius(rounding);
        if let Some(border) = self.border {
            shape = shape.border(border);
        }
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![
                Paint::Shape(shape.into()),
                Paint::Text {
                    text: label.to_owned(),
                    position,
                    size,
                    weight,
                    wrap_width: f32::INFINITY,
                    color: self.text_color.unwrap_or(style.text_color),
                },
            ],
        );
        response
    }
}
