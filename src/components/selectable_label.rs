use super::{
    rich_text::RichText,
    text_block::{text_builders, Block, BlockOutput, LinkReport},
    Response, Ui, Widget,
};
use crate::Id;
use std::hash::Hash;

/// Static text that can be selected with the pointer and copied, like text in a browser or a
/// native app.
///
/// A press and drag selects whole grapheme clusters; a double click selects a word, a triple
/// click the paragraph, Shift+click extends, Ctrl/Cmd+A selects everything and
/// Ctrl/Cmd+C (or Ctrl+Insert) copies exactly the selected characters: explicit line breaks
/// stay, breaks that only wrapping added do not, and a text shortened with an ellipsis copies
/// in full. The label takes keyboard focus as one Tab stop so the shortcuts work, and shows
/// no caret: it is not editable. The pointer is an I-beam over it.
///
/// Several labels inside [`Ui::selection_scope`] select as one document. A plain
/// [`Text`](crate::Text) is unaffected and stays free of any of this; inside a clickable row,
/// either use `Text` or turn the selection off with `.selectable(false)`.
pub struct SelectableLabel {
    block: Block,
    copy_urls: bool,
}

impl SelectableLabel {
    pub fn new(text: impl Into<String>) -> Self {
        let mut block = Block::new("selectable", text.into());
        block.selectable = true;
        Self {
            block,
            copy_urls: false,
        }
    }
    /// A label that follows a [`RichText`]: mixed styles, and links that work on their own.
    pub fn rich(text: RichText) -> Self {
        let (source, spans) = text.into_parts();
        let mut label = Self::new(source);
        label.block.spans = spans;
        label
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.block.id = Some(Id::new(source));
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.block.enabled = enabled;
        self
    }
    /// Turn selection off, leaving a plain label with the same layout. For text inside rows
    /// that handle clicks themselves.
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.block.selectable = selectable;
        self
    }
    /// Copy the address after each link that is part of a copied selection, as
    /// `text (https://…)`. Off by default: a copy holds the visible text.
    pub fn copy_url_in_selection(mut self, copy: bool) -> Self {
        self.copy_urls = copy;
        self
    }
    /// A small copy icon right of the text that copies all of it. It turns into a check
    /// mark for a moment afterwards and is not a Tab stop.
    pub fn copy_button(mut self) -> Self {
        self.block.copy_button = true;
        self
    }
    /// The Copy / Select all menu on a secondary click (on by default).
    pub fn context_menu(mut self, menu: bool) -> Self {
        self.block.menu = menu;
        self
    }
    /// Open a link's address with the system handler on activation; see
    /// [`Hyperlink::open_in_browser`](super::Hyperlink::open_in_browser).
    pub fn open_links_in_browser(mut self) -> Self {
        self.block.open = Some(super::UrlPolicy::default());
        self
    }
    pub fn link_style(mut self, style: super::HyperlinkStyle) -> Self {
        self.block.link_style = style;
        self
    }
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.block.tooltip = Some(text.into());
        self
    }

    /// Text color; the theme's text color by default.
    pub fn color(mut self, color: crate::Color) -> Self {
        self.block.text.color = Some(color);
        self
    }

    text_builders!(block.text);

    /// Show the label and get the links that were activated.
    pub fn show(self, ui: &mut Ui<'_>) -> LabelOutput {
        ui.layout_item(|ui| self.run(ui))
    }

    fn run(mut self, ui: &mut Ui<'_>) -> LabelOutput {
        self.block.copy_urls = self.copy_urls;
        let BlockOutput {
            response,
            links,
            copied,
        } = self.block.show(ui);
        LabelOutput {
            response,
            links,
            copied,
        }
    }
}

/// The result of [`SelectableLabel::show`].
#[derive(Clone, Debug)]
pub struct LabelOutput {
    pub response: Response,
    /// Every link of the text, with its own response and this pass's activation.
    pub links: Vec<LinkReport>,
    /// The copy button copied the text during this pass.
    pub copied: bool,
}

impl LabelOutput {
    /// The first link that was activated this pass.
    pub fn activated(&self) -> Option<&LinkReport> {
        self.links.iter().find(|l| l.activation.is_some())
    }
}

impl Widget for SelectableLabel {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        self.run(ui).response
    }
}

impl Ui<'_> {
    /// Static text the user can select and copy; see [`SelectableLabel`].
    pub fn selectable_label(&mut self, text: impl Into<String>) -> Response {
        self.add(SelectableLabel::new(text))
    }
    /// A selectable label with a copy button.
    pub fn copyable_label(&mut self, text: impl Into<String>) -> Response {
        self.add(SelectableLabel::new(text).copy_button())
    }
    /// Selectable text with styled spans and independent links. Returns the response of
    /// the text and the links; see [`RichText`].
    pub fn rich_label(&mut self, text: RichText) -> LabelOutput {
        SelectableLabel::rich(text).show(self)
    }
}
