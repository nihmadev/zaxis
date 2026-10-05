//! A flat list of styled spans over one string: the model behind inline links and mixed
//! styles. Spans never nest and never overlap, and their ranges lie on grapheme cluster
//! boundaries, so a span can neither split a cluster nor start inside a combining sequence.
//! The font size is one value for the whole text.

use super::edit_buffer::{is_boundary, snap_down, snap_up};
use crate::{Color, FontWeight, Id};
use std::ops::Range;

/// When a link is underlined.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Underline {
    Always,
    /// Appears while the pointer is over the link or it has keyboard focus.
    #[default]
    Hover,
    Never,
}

/// Look of one span. `None` keeps the surrounding text's value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpanStyle {
    pub color: Option<Color>,
    pub weight: Option<FontWeight>,
    /// Set in the monospace family.
    pub monospace: bool,
    /// A plain underline. Links follow their [`Underline`] policy instead.
    pub underline: bool,
}

impl SpanStyle {
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn weight(mut self, weight: FontWeight) -> Self {
        self.weight = Some(weight);
        self
    }
    pub fn bold(self) -> Self {
        self.weight(FontWeight::BOLD)
    }
    pub fn monospace(mut self) -> Self {
        self.monospace = true;
        self
    }
    pub fn underline(mut self, underline: bool) -> Self {
        self.underline = underline;
        self
    }
}

/// A link inside rich text. It is focusable and clickable on its own; the library reports
/// its activation and never opens anything unless the widget says so.
#[derive(Clone, Debug, PartialEq)]
pub struct LinkTarget {
    /// Stable identity of the link; defaults to its position among the links of the text.
    pub id: Option<Id>,
    pub url: Option<String>,
    pub visited: bool,
    pub enabled: bool,
    pub underline: Option<Underline>,
    pub tooltip: Option<String>,
}

impl LinkTarget {
    pub fn new() -> Self {
        Self {
            id: None,
            url: None,
            visited: false,
            enabled: true,
            underline: None,
            tooltip: None,
        }
    }
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }
    pub fn id(mut self, id: impl std::hash::Hash) -> Self {
        self.id = Some(Id::new(id));
        self
    }
    pub fn visited(mut self, visited: bool) -> Self {
        self.visited = visited;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn underline(mut self, underline: Underline) -> Self {
        self.underline = Some(underline);
        self
    }
    /// Shown on hover instead of the url.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

impl Default for LinkTarget {
    fn default() -> Self {
        Self::new()
    }
}

/// A styled byte range, with an optional link.
#[derive(Clone, Debug, PartialEq)]
pub struct Span {
    pub range: Range<usize>,
    pub style: SpanStyle,
    pub link: Option<LinkTarget>,
}

impl Span {
    pub fn new(range: Range<usize>) -> Self {
        Self {
            range,
            style: SpanStyle::default(),
            link: None,
        }
    }
    pub fn style(mut self, style: SpanStyle) -> Self {
        self.style = style;
        self
    }
    pub fn link(mut self, link: LinkTarget) -> Self {
        self.link = Some(link);
        self
    }
}

/// Text with styled spans, built piece by piece or from explicit ranges.
///
/// ```
/// # use zaxis::RichText;
/// let text = RichText::new().text("see ").link("docs", "https://example.com").text(" for details");
/// assert_eq!(text.as_str(), "see docs for details");
/// assert_eq!(text.spans()[0].range, 4..8);
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RichText {
    text: String,
    spans: Vec<Span>,
}

impl RichText {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spans over `text`. A range that is out of bounds is clamped, one that cuts a grapheme
    /// cluster is widened to whole clusters, and empty or overlapping ranges are dropped; each
    /// of these is reported as an invalid value rather than a panic.
    #[track_caller]
    pub fn from_spans(text: impl Into<String>, spans: impl IntoIterator<Item = Span>) -> Self {
        let text = text.into();
        let mut kept: Vec<Span> = Vec::new();
        let mut at = 0;
        for mut span in spans {
            let (start, end) = (span.range.start, span.range.end.min(text.len()));
            let ok = start < end && is_boundary(&text, start) && is_boundary(&text, end);
            let snapped = snap_down(&text, start.min(end))..snap_up(&text, end);
            if span.range != (start..end) || !ok {
                crate::context::invalid_value(
                    "RichText::from_spans",
                    format!(
                        "range {:?} is not on cluster boundaries inside 0..{}",
                        span.range,
                        text.len()
                    ),
                );
            }
            span.range = snapped;
            if span.range.start < at || span.range.is_empty() {
                continue;
            }
            at = span.range.end;
            kept.push(span);
        }
        Self { text, spans: kept }
    }

    fn push(mut self, text: &str, style: SpanStyle, link: Option<LinkTarget>) -> Self {
        let start = self.text.len();
        self.text.push_str(text);
        if !text.is_empty() && (link.is_some() || style != SpanStyle::default()) {
            self.spans.push(Span {
                range: start..self.text.len(),
                style,
                link,
            });
        }
        self
    }

    /// Plain text in the surrounding style.
    pub fn text(self, text: &str) -> Self {
        self.push(text, SpanStyle::default(), None)
    }
    pub fn styled(self, text: &str, style: SpanStyle) -> Self {
        self.push(text, style, None)
    }
    pub fn bold(self, text: &str) -> Self {
        self.push(text, SpanStyle::default().bold(), None)
    }
    pub fn code(self, text: &str) -> Self {
        self.push(text, SpanStyle::default().monospace(), None)
    }
    pub fn colored(self, text: &str, color: Color) -> Self {
        self.push(text, SpanStyle::default().color(color), None)
    }
    pub fn link(self, text: &str, url: impl Into<String>) -> Self {
        self.push(text, SpanStyle::default(), Some(LinkTarget::new().url(url)))
    }
    pub fn link_with(self, text: &str, link: LinkTarget) -> Self {
        self.push(text, SpanStyle::default(), Some(link))
    }

    pub fn as_str(&self) -> &str {
        &self.text
    }
    pub fn spans(&self) -> &[Span] {
        &self.spans
    }
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
    pub(crate) fn into_parts(self) -> (String, Vec<Span>) {
        (self.text, self.spans)
    }
}
