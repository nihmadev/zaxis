use crate::{context::Paint, Color};

use super::{font_size, Response, Ui, Widget};

/// A text label with newline handling, kerning, and optional character wrapping.
pub struct Text {
    text: String,
    size: Option<f32>,
    weight: Option<crate::FontWeight>,
    color: Option<Color>,
    wrap: bool,
    muted: bool,
    style: crate::TextStyle,
    role: crate::TypographyRole,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            size: None,
            weight: None,
            color: None,
            wrap: true,
            muted: false,
            style: Default::default(),
            role: Default::default(),
        }
    }
    pub fn style(mut self, style: crate::TextStyle) -> Self {
        self.style = style;
        self
    }
    pub fn typography(mut self, role: crate::TypographyRole) -> Self {
        self.role = role;
        self
    }
    #[track_caller]
    pub fn size(mut self, size: f32) -> Self {
        self.size = super::sanitize::positive("Text::size", size).or(self.size);
        self
    }
    /// The font file weight for this text; overrides the style and the role.
    pub fn weight(mut self, weight: crate::FontWeight) -> Self {
        self.weight = Some(weight);
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
        let mut style = ui.style().text;
        style.merge(self.style);
        let inherited = match self.role {
            crate::TypographyRole::Body => ui.style().font_size,
            crate::TypographyRole::Small => ui.style().typography.small,
            crate::TypographyRole::Heading => ui.style().typography.heading,
            crate::TypographyRole::Title => ui.style().typography.title,
        };
        let size = font_size(self.size.or(style.size).unwrap_or(inherited));
        let weight = self
            .weight
            .or(style.weight)
            .unwrap_or_else(|| ui.style().typography.weights.role(self.role));
        let color = self.color.or(style.color).unwrap_or(if !ui.is_enabled() {
            ui.style().disabled_text
        } else if self.muted {
            style.muted.unwrap_or(ui.style().muted_text)
        } else {
            ui.style().text_color
        });
        let wrap_width = if self.wrap {
            ui.available_width()
        } else {
            f32::INFINITY
        };
        let (text_size, wrap_width) = ui
            .context
            .measure_text_layout(&self.text, size, weight, wrap_width);
        let rect = ui.allocate_space(text_size);
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![Paint::Text {
                text: self.text,
                position: rect.min,
                size,
                weight,
                wrap_width,
                color: super::appearance::alpha(color, ui.style().opacity),
            }],
        );
        ui.response(id, rect, false)
    }
}

impl Ui<'_> {
    /// Text at a theme size. `heading`, `title` and `small` are shortcuts for the roles.
    pub fn typography(&mut self, role: crate::TypographyRole, text: impl Into<String>) -> Response {
        self.add(Text::new(text).typography(role))
    }
    pub fn heading(&mut self, text: impl Into<String>) -> Response {
        self.typography(crate::TypographyRole::Heading, text)
    }
    pub fn title(&mut self, text: impl Into<String>) -> Response {
        self.typography(crate::TypographyRole::Title, text)
    }
    pub fn small(&mut self, text: impl Into<String>) -> Response {
        self.typography(crate::TypographyRole::Small, text)
    }
    pub fn muted(&mut self, text: impl Into<String>) -> Response {
        self.add(Text::new(text).muted())
    }
    pub fn label(&mut self, text: impl Into<String>) -> Response {
        self.add(Text::new(text))
    }
}
