use crate::{context::Paint, Color};

use super::{font_size, Response, Ui, Widget};

/// A text label with newline handling, kerning, and optional character wrapping.
///
/// The family is independent of size and weight: [`Text::monospace`] sets the text in the
/// code font on a fixed cell grid, [`Text::tabular_numbers`] gives proportional text digits
/// of equal width. Both are separate from [`Text::wrap`], which stays on by default; call
/// `wrap(false)` for code and columns so lines are never broken, only clipped by the parent.
pub struct Text {
    text: String,
    size: Option<f32>,
    weight: Option<crate::FontWeight>,
    color: Option<Color>,
    wrap: bool,
    family: Option<crate::TextFamily>,
    tabular: Option<bool>,
    tab: Option<u16>,
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
            family: None,
            tabular: None,
            tab: None,
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
    /// Set the text in the monospace family (bundled JetBrains Mono by default). Overrides
    /// the style and the role; see [`Ui::monospace_metrics`] for the cell size.
    pub fn monospace(self) -> Self {
        self.family(crate::TextFamily::Monospace)
    }
    /// The font family of this text; overrides the style and the role.
    pub fn family(mut self, family: crate::TextFamily) -> Self {
        self.family = Some(family);
        self
    }
    /// Tabular figures: every digit has the width of the widest one, so numbers in a column
    /// line up. Uses the font's OpenType `tnum` feature; a font without it (or the monospace
    /// family, which is already tabular) renders unchanged. Never fakes alignment by padding.
    pub fn tabular_numbers(mut self, tabular: bool) -> Self {
        self.tabular = Some(tabular);
        self
    }
    /// Width of a tab character in cells (space advances), at least 1; default 8.
    pub fn tab_size(mut self, cells: u16) -> Self {
        self.tab = Some(cells.max(1));
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
            role => ui.style().typography.size(role),
        };
        let size = font_size(self.size.or(style.size).unwrap_or(inherited));
        let weight = self
            .weight
            .or(style.weight)
            .unwrap_or_else(|| ui.style().typography.weights.role(self.role));
        let family = self.family.or(style.family).unwrap_or(match self.role {
            crate::TypographyRole::Code => crate::TextFamily::Monospace,
            _ => crate::TextFamily::Proportional,
        });
        let font = crate::text::TextFont::new(
            weight,
            family,
            self.tabular.or(style.tabular_numbers).unwrap_or(false),
        );
        let tab = self.tab.unwrap_or(crate::text::DEFAULT_TAB);
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
            .measure_text_layout_tab(&self.text, size, font, wrap_width, tab);
        let rect = ui.allocate_space(text_size);
        ui.context.paint(
            id,
            ui.window,
            ui.clip,
            vec![Paint::text(
                self.text,
                rect.min,
                size,
                font,
                wrap_width,
                tab,
                super::appearance::alpha(color, ui.style().opacity),
            )],
        );
        ui.response(id, rect, false)
    }
}

impl Ui<'_> {
    /// Text at a theme size. `heading`, `title` and `small` are shortcuts for the roles.
    pub fn typography(&mut self, role: crate::TypographyRole, text: impl Into<String>) -> Response {
        self.add(Text::new(text).typography(role))
    }
    /// Source text, a hash or a log line in the monospace family at the `code` size. Set
    /// `wrap(false)` on a [`Text`] to keep long lines whole.
    pub fn code(&mut self, text: impl Into<String>) -> Response {
        self.add(Text::new(text).typography(crate::TypographyRole::Code))
    }
    /// The cell width and line height of the code role (`Typography::code`, its weight and
    /// the monospace family), the numbers a code view needs for `content_width` and
    /// [`ScrollArea::show_rows`](crate::ScrollArea::show_rows) row heights.
    pub fn monospace_metrics(&mut self) -> crate::MonospaceMetrics {
        let typography = self.style().typography;
        self.context
            .monospace_metrics(typography.code, typography.weights.code)
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
