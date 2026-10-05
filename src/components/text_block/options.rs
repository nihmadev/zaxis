//! Typography of static text, resolved the way [`Text`](crate::Text) resolves it: the same
//! theme size, weight, family and colors, so a selectable label looks exactly like a label.

use crate::{
    components::{font_size, Ui},
    text::{TextFont, DEFAULT_TAB},
    Color, FontWeight, TextFamily, TextStyle, TypographyRole,
};

#[derive(Clone, Debug)]
pub(crate) struct TextOptions {
    pub size: Option<f32>,
    pub weight: Option<FontWeight>,
    pub color: Option<Color>,
    pub family: Option<TextFamily>,
    pub tabular: Option<bool>,
    pub tab: Option<u16>,
    pub muted: bool,
    pub wrap: bool,
    pub max_lines: Option<usize>,
    pub style: TextStyle,
    pub role: TypographyRole,
}

impl Default for TextOptions {
    fn default() -> Self {
        Self {
            size: None,
            weight: None,
            color: None,
            family: None,
            tabular: None,
            tab: None,
            muted: false,
            wrap: true,
            max_lines: None,
            style: TextStyle::default(),
            role: TypographyRole::Body,
        }
    }
}

/// Resolved typography of a block.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Look {
    pub size: f32,
    pub font: TextFont,
    pub tab: u16,
    pub color: Color,
}

impl TextOptions {
    pub fn resolve(&self, ui: &Ui<'_>) -> Look {
        let mut style = ui.style().text;
        style.merge(self.style);
        let inherited = match self.role {
            TypographyRole::Body => ui.style().font_size,
            role => ui.style().typography.size(role),
        };
        let size = font_size(self.size.or(style.size).unwrap_or(inherited));
        let weight = self
            .weight
            .or(style.weight)
            .unwrap_or_else(|| ui.style().typography.weights.role(self.role));
        let family = self.family.or(style.family).unwrap_or(match self.role {
            TypographyRole::Code => TextFamily::Monospace,
            _ => TextFamily::Proportional,
        });
        let font = TextFont::new(
            weight,
            family,
            self.tabular.or(style.tabular_numbers).unwrap_or(false),
        );
        let color = self.color.or(style.color).unwrap_or(if !ui.is_enabled() {
            ui.style().disabled_text
        } else if self.muted {
            style.muted.unwrap_or(ui.style().muted_text)
        } else {
            ui.style().text_color
        });
        Look {
            size,
            font,
            tab: self.tab.unwrap_or(DEFAULT_TAB),
            color,
        }
    }
}

/// Typography builders shared by every static text widget; each forwards to `self.text`.
macro_rules! text_builders {
    ($($field:ident).+) => {
        /// Theme text style; explicit builders below still win.
        pub fn text_style(mut self, style: $crate::TextStyle) -> Self {
            self.$($field).+.style = style;
            self
        }
        pub fn typography(mut self, role: $crate::TypographyRole) -> Self {
            self.$($field).+.role = role;
            self
        }
        #[track_caller]
        pub fn size(mut self, size: f32) -> Self {
            self.$($field).+.size = $crate::components::sanitize::positive("size", size).or(self.$($field).+.size);
            self
        }
        pub fn weight(mut self, weight: $crate::FontWeight) -> Self {
            self.$($field).+.weight = Some(weight);
            self
        }
        pub fn muted(mut self) -> Self {
            self.$($field).+.muted = true;
            self
        }
        /// Set the text in the monospace family. Spaces and tabs are kept exactly.
        pub fn monospace(mut self) -> Self {
            self.$($field).+.family = Some($crate::TextFamily::Monospace);
            self
        }
        pub fn family(mut self, family: $crate::TextFamily) -> Self {
            self.$($field).+.family = Some(family);
            self
        }
        pub fn tabular_numbers(mut self, tabular: bool) -> Self {
            self.$($field).+.tabular = Some(tabular);
            self
        }
        pub fn tab_size(mut self, cells: u16) -> Self {
            self.$($field).+.tab = Some(cells.max(1));
            self
        }
        /// Break lines at the container width (default). Without it every line stays whole
        /// and the parent clips it.
        pub fn wrap(mut self, wrap: bool) -> Self {
            self.$($field).+.wrap = wrap;
            self
        }
        /// Keep one line and end it with an ellipsis when it does not fit. Selection and
        /// copy still use the whole text.
        pub fn truncate(mut self, truncate: bool) -> Self {
            self.$($field).+.max_lines = truncate.then_some(1);
            self
        }
        /// At most `lines` visual lines, ending with an ellipsis when the text is longer.
        pub fn max_lines(mut self, lines: usize) -> Self {
            self.$($field).+.max_lines = Some(lines.max(1));
            self
        }
    };
}
pub(crate) use text_builders;
