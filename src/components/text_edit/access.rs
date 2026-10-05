//! What a field tells assistive technology and what it accepts from it.
//!
//! The text is published as one run per visual line, built from the layouts the field is
//! painted with, so a caret position a screen reader gets is the position the edit buffer
//! uses. Requests come back as ordinary edit events: a new value or a replaced selection
//! goes through the buffer and the undo history like typed text.
use super::{area::AreaState, doc::Env, text_input::Fingerprint, *};
use crate::{
    accessibility::text::{TextModel, TextRef},
    context::Context,
    text::TextLayout,
    AccessAction, AccessActionKind as Kind, AccessRole,
};
use std::sync::Arc;

/// Longest text of a multi-line field published as the node's value. The lines themselves
/// are published around the viewport whatever the length.
const MAX_VALUE: usize = 256 * 1024;

/// The visual lines of a field and where its text starts relative to the field's rect.
pub(super) struct Lines {
    pub text: TextRef,
    pub origin: Vec2,
    /// Anchor and focus, as byte offsets into the published text.
    pub selection: (usize, usize),
}

/// The line model of a single-line field with the layout it was built from.
pub(crate) type LineCache = Option<(Arc<TextLayout>, TextRef)>;

/// Everything the lines of a text area depend on.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct AreaKey {
    fingerprint: Fingerprint,
    first: usize,
    last: usize,
    top: u32,
    env: Env,
}

/// Requests from assistive technology for field `id`, as edit events.
pub(super) fn requests(ctx: &mut Context, id: Id, events: &mut Vec<TextEditInput>) {
    for action in ctx.take_access_actions(id) {
        events.push(match action {
            AccessAction::SetValue(text) => TextEditInput::SetValue(text),
            AccessAction::ReplaceSelectedText(text) => TextEditInput::Commit(text),
            AccessAction::SetTextSelection { anchor, focus } => {
                TextEditInput::Select(anchor, focus)
            }
            _ => continue,
        });
    }
}

/// The one line of a single-line field. `shown` is the text on screen (with a composition
/// string in place) and `layout` its shaping; the model is rebuilt only when that changes.
pub(super) fn line(cache: &mut LineCache, shown: &str, layout: &Arc<TextLayout>) -> TextRef {
    if let Some((_, text)) = cache.as_ref().filter(|(old, _)| Arc::ptr_eq(old, layout)) {
        return text.clone();
    }
    let mut model = TextModel::default();
    model.push_lines(shown, 0, &layout.lines, Vec2::ZERO, "");
    let text = TextRef::from(model);
    *cache = Some((Arc::clone(layout), text.clone()));
    text
}

impl TextEdit<'_> {
    /// The lines of paragraphs `first..=last` of a text area, in content coordinates:
    /// the paragraphs that were shaped for painting. Text outside the viewport is not
    /// laid out, so it is in the node's value but not in its lines.
    pub(super) fn area_lines(
        &self,
        ctx: &mut Context,
        area: &mut AreaState,
        (first, last): (usize, usize),
        composing: bool,
    ) -> TextRef {
        let doc = &mut area.doc;
        let key = AreaKey {
            fingerprint: Fingerprint::of(self.text),
            first,
            last,
            top: doc.top(first).to_bits(),
            env: doc.env(),
        };
        if let Some((_, text)) = area
            .access
            .as_ref()
            .filter(|(old, _)| *old == key && !composing)
        {
            return text.clone();
        }
        let text: &str = self.text;
        let mut model = TextModel::default();
        // A composition string makes its paragraph longer than the stored one; the
        // paragraphs after it move by the difference so the lines stay contiguous.
        let mut shift = 0;
        for i in first..=last {
            let layout = doc.layout(ctx, text, i);
            let top = doc.top(i);
            let para = &doc.paras[i];
            let shown = doc.shown(text, i);
            model.push_lines(
                shown,
                para.start + shift,
                &layout.lines,
                Vec2::new(0.0, top),
                &text[para.end()..para.next_start()],
            );
            shift += shown.len().saturating_sub(para.len);
        }
        let text = TextRef::from(model);
        area.access = (!composing).then(|| (key, text.clone()));
        text
    }

    /// Publish the field. `shown` is the text on screen when it differs from the bound
    /// string (a composition in progress).
    pub(super) fn describe(
        &self,
        ui: &mut Ui<'_>,
        id: Id,
        rect: Rect,
        lines: Option<Lines>,
        shown: Option<&str>,
    ) {
        let multiline = self.area.is_some();
        let role = if multiline {
            AccessRole::MultilineTextInput
        } else {
            AccessRole::TextInput
        };
        let invalid = ui.field_status(self.status) == crate::SemanticStatus::Error;
        ui.a11y(id, rect, role, |node| {
            node.placeholder(self.placeholder.as_str())
                .read_only(self.read_only)
                .disabled(!self.enabled)
                .invalid(invalid)
                .action(Kind::SetTextSelection);
            if !self.read_only {
                node.action(Kind::SetValue)
                    .action(Kind::ReplaceSelectedText);
            }
            let value = shown.unwrap_or(self.text);
            if !multiline || value.len() <= MAX_VALUE {
                node.value(value);
            }
            if let Some(lines) = lines {
                node.text(lines.text, Some(lines.selection))
                    .text_origin(lines.origin);
            }
        });
    }
}
