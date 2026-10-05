//! Retained state of one static text block, owned by the [`Context`](crate::Context) under
//! the block's stable [`Id`]. Everything here is rebuilt or dropped when the block stops
//! being drawn, and replaced when its text changes.

use crate::time::Instant;
use crate::{
    components::{
        edit_buffer::{snap_down, snap_up},
        text_edit::doc::{Doc, Env},
    },
    Id, Vec2,
};
use std::ops::Range;

/// A link of the text as the last pass built it.
#[derive(Clone, Debug)]
pub(crate) struct LinkInfo {
    pub id: Id,
    /// Range in source bytes.
    pub range: Range<usize>,
    pub url: Option<String>,
}

/// What an open context menu was opened on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MenuTarget {
    Text,
    Link(usize),
}

pub(crate) struct BlockState {
    /// The full source text; selection and copy always use it.
    pub text: String,
    /// The text shown when the block is truncated: a prefix of `text` and an ellipsis.
    pub display: Option<String>,
    /// Source byte where the shown text ends while truncated.
    pub cut: Option<usize>,
    pub doc: Doc,
    /// Identity of the text, wrap width and typography the document was built for.
    pub built: Option<Id>,
    pub rev: u64,
    pub scope: Id,
    pub ord: usize,
    pub size: Vec2,
    pub links: Vec<LinkInfo>,
    pub menu: Option<MenuTarget>,
    pub copied: Option<Instant>,
    pub last_frame: u64,
    /// The lines last described to assistive technology, with the layouts they came from.
    pub access: Option<super::access::Lines>,
}

impl BlockState {
    pub fn new(text: &str, env: Env) -> Self {
        Self {
            text: text.to_owned(),
            display: None,
            cut: None,
            doc: Doc::new(text, env),
            built: None,
            rev: 0,
            scope: Id::new(0),
            ord: 0,
            size: Vec2::ZERO,
            links: Vec::new(),
            menu: None,
            copied: None,
            last_frame: 0,
            access: None,
        }
    }

    /// The text the document holds and the block paints.
    pub fn shown(&self) -> &str {
        self.display.as_deref().unwrap_or(&self.text)
    }

    /// Take a changed source text. Returns whether it changed.
    pub fn set_text(&mut self, text: &str) -> bool {
        if self.text == text {
            return false;
        }
        self.text.clear();
        self.text.push_str(text);
        self.rev += 1;
        self.display = None;
        self.cut = None;
        self.built = None;
        true
    }

    /// A source byte clamped onto a grapheme boundary of the current text.
    pub fn clamp(&self, byte: usize) -> usize {
        snap_down(&self.text, byte.min(self.text.len()))
    }

    /// Source bytes `range` as the text shown: everything past the cut becomes the ellipsis.
    pub fn to_display(&self, range: Range<usize>) -> Range<usize> {
        let Some(cut) = self.cut else {
            return range;
        };
        let end = self.shown().len();
        let map = |b: usize| if b > cut { end } else { b };
        range.start.min(cut)..map(range.end)
    }

    /// A byte of the shown text as a source byte; the ellipsis stands for the hidden rest.
    pub fn to_source(&self, byte: usize) -> usize {
        match self.cut {
            Some(cut) if byte > cut => self.text.len(),
            _ => byte,
        }
    }

    /// The visible text of `range`, with the address of every link it touches appended as
    /// ` (url)` when `urls` is set. Bytes are clamped onto grapheme boundaries.
    pub fn copy_range(&self, range: Range<usize>, urls: bool) -> String {
        let len = self.text.len();
        let a = snap_down(&self.text, range.start.min(len));
        let b = snap_up(&self.text, range.end.min(len));
        if a >= b {
            return String::new();
        }
        let mut out = String::with_capacity(b - a);
        let mut at = a;
        if urls {
            for link in &self.links {
                let Some(url) = link.url.as_deref() else {
                    continue;
                };
                if link.range.end > a && link.range.start < b {
                    let end = link.range.end.min(b);
                    out.push_str(&self.text[at..end]);
                    out.push_str(" (");
                    out.push_str(url);
                    out.push(')');
                    at = end;
                }
            }
        }
        out.push_str(&self.text[at..b]);
        out
    }
}
