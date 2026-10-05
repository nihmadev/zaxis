use super::{
    rich_text::{LinkTarget, Span, Underline},
    text_block::{text_builders, Block, BlockOutput, LinkReport},
    Response, Ui, Widget,
};
use crate::Id;
use std::hash::Hash;

mod launch;
mod style;
mod url;

pub use style::HyperlinkStyle;
pub use url::{UrlError, UrlPolicy};

/// How a link was activated.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LinkActivation {
    /// A primary click, or Enter or Space on the focused link.
    Primary,
    /// A middle click or a Ctrl+click: the usual "open in a new place" gesture. The
    /// application decides what that means.
    Secondary,
}

/// The result of showing a [`Hyperlink`] with [`Hyperlink::show`].
#[derive(Clone, Debug)]
pub struct HyperlinkOutput {
    pub response: Response,
    /// The activation of this pass, reported once per user action.
    pub activated: Option<LinkActivation>,
    /// The address of the link, whether or not it was activated.
    pub url: Option<String>,
}

/// A link as a widget of its own: accent colored text that underlines on hover, is
/// reachable with Tab, and activates on release over it (or Enter and Space). A link that
/// is longer than the line wraps, and every line fragment is clickable.
///
/// Showing a link only reports the click: [`Response::link_activation`] tells how it was
/// activated, and the application decides what happens. Opening the address in the system
/// browser is opt-in with [`Hyperlink::open_in_browser`], and checks the scheme first.
///
/// ```
/// # use zaxis::*;
/// # let mut ctx = Context::new();
/// # ctx.run(|c| { Root::new().show(c, |ui| {
/// if ui.hyperlink_to("Documentation", "https://example.com/docs").clicked() {
///     // navigate, or open it yourself
/// }
/// # }); });
/// ```
pub struct Hyperlink {
    text: String,
    link: LinkTarget,
    block: Block,
}

impl Hyperlink {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let mut block = Block::new("hyperlink", text.clone());
        block.link_is_block = true;
        Self {
            block,
            text,
            link: LinkTarget::new(),
        }
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.link.url = Some(url.into());
        self
    }
    /// The application knows the link was followed; it gets the visited color.
    pub fn visited(mut self, visited: bool) -> Self {
        self.link.visited = visited;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.block.enabled = enabled;
        self
    }
    /// Shown on hover. Without one the full address is shown, when there is one.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.link.tooltip = Some(text.into());
        self
    }
    pub fn underline(mut self, underline: Underline) -> Self {
        self.link.underline = Some(underline);
        self
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.block.id = Some(Id::new(source));
        self
    }
    pub fn style(mut self, style: HyperlinkStyle) -> Self {
        self.block.link_style = style;
        self
    }
    /// Open the address with the system handler when the link is activated, after checking
    /// its scheme against the default allowlist (`https`, `http`, `mailto`). Without this the
    /// library never opens anything.
    pub fn open_in_browser(mut self) -> Self {
        self.block.open = Some(UrlPolicy::default());
        self
    }
    /// Like [`Self::open_in_browser`] with an explicit allowlist.
    pub fn open_with(mut self, policy: UrlPolicy) -> Self {
        self.block.open = Some(policy);
        self
    }
    /// Let the pointer select the link's text and copy it. Off by default: a link is a
    /// control, and a press on it must not start a selection.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.block.selectable = selectable;
        self
    }
    /// The Copy menu on a secondary click (on by default).
    pub fn context_menu(mut self, menu: bool) -> Self {
        self.block.menu = menu;
        self
    }

    text_builders!(block.text);

    /// Show the link and get its activation and address.
    pub fn show(self, ui: &mut Ui<'_>) -> HyperlinkOutput {
        ui.layout_item(|ui| self.run(ui))
    }

    fn run(mut self, ui: &mut Ui<'_>) -> HyperlinkOutput {
        let len = self.text.len();
        let url = self.link.url.clone();
        if len > 0 {
            self.block.spans = vec![Span::new(0..len).link(self.link)];
        }
        let BlockOutput {
            response, links, ..
        } = self.block.show(ui);
        let report: Option<LinkReport> = links.into_iter().next();
        HyperlinkOutput {
            response,
            activated: report.as_ref().and_then(|r| r.activation),
            url,
        }
    }
}

impl Widget for Hyperlink {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        self.run(ui).response
    }
}

impl Ui<'_> {
    /// A link without an address; see [`Hyperlink`].
    pub fn hyperlink(&mut self, text: impl Into<String>) -> Response {
        self.add(Hyperlink::new(text))
    }
    /// A link with an address, reported in [`Response::link_activation`] and never opened
    /// by the library; see [`Hyperlink::open_in_browser`].
    pub fn hyperlink_to(&mut self, text: impl Into<String>, url: impl Into<String>) -> Response {
        self.add(Hyperlink::new(text).url(url))
    }
}
