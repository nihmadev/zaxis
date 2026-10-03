use crate::{context::Paint, Color};

use super::{font_size, Response, Ui, Widget};

/// A text label with newline handling, kerning, and optional character wrapping.
pub struct Text {
    text: String,
    size: Option<f32>,
    color: Option<Color>,
    wrap: bool,
    muted: bool,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: None,
            color: None,
            wrap: true,
            muted: false,
        }
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }
}

impl Widget for Text {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = ui.next_id("text");
        let size = font_size(self.size.unwrap_or(ui.style().font_size));
        let color = self.color.unwrap_or(if self.muted || !ui.is_enabled() {
            ui.style().muted_text
        } else {
            ui.style().text_color
        });
        let wrap_width = if self.wrap {
            ui.available_width()
        } else {
            f32::INFINITY
        };
        let text_size = ui.context.measure_text(&self.text, size, wrap_width);
        let rect = ui.allocate_space(text_size);
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![Paint::Text {
                text: self.text,
                position: rect.min,
                size,
                wrap_width,
                color,
            }],
        );
        ui.response(id, rect, false)
    }
}

impl Ui<'_> {
    pub fn muted(&mut self, text: impl Into<String>) -> Response {
        self.add(Text::new(text).muted())
    }
    pub fn label(&mut self, text: impl Into<String>) -> Response {
        self.add(Text::new(text))
    }
}
