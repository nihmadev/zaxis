//! One engine for static text that does more than paint: it can be selected and copied,
//! carry styled spans, and hold links. [`Hyperlink`](super::Hyperlink),
//! [`SelectableLabel`](super::SelectableLabel) and [`Ui::rich_label`](super::Ui::rich_label)
//! are thin builders over it; a plain [`Text`](super::Text) never goes through here, so it
//! pays for none of this.
//!
//! The text is a paragraph table ([`Doc`](super::text_edit::doc::Doc), the same one
//! [`TextEdit`](super::TextEdit) uses) kept in the context under the block's [`Id`]. Hit
//! testing, selection backing, underlines and the painted glyphs all read its cached layouts,
//! so what is under the pointer is what is drawn there.

use super::{
    hyperlink::{HyperlinkStyle, LinkActivation, UrlPolicy},
    rich_text::{Span, Underline},
    Response, Tooltip, Ui,
};
use crate::{
    context::{HitAction, HitRegion},
    Id, Rect, Vec2,
};
pub(crate) use state::BlockState;

mod access;
mod copy_button;
mod fit;
mod layout;
mod links;
mod menu;
pub(crate) mod options;
mod paint;
mod select;
pub(crate) mod state;

pub(crate) use options::{text_builders, TextOptions};

/// What a block was asked to be.
pub(crate) struct Block {
    pub kind: &'static str,
    pub id: Option<Id>,
    pub source: String,
    pub spans: Vec<Span>,
    pub text: TextOptions,
    pub selectable: bool,
    pub enabled: bool,
    pub underline: Underline,
    pub link_style: HyperlinkStyle,
    pub tooltip: Option<String>,
    pub menu: bool,
    pub copy_button: bool,
    pub copy_urls: bool,
    pub open: Option<UrlPolicy>,
    /// The only link is the block itself, so it takes the block's ID (a standalone link).
    pub link_is_block: bool,
}

impl Block {
    pub fn new(kind: &'static str, source: String) -> Self {
        Self {
            kind,
            id: None,
            source,
            spans: Vec::new(),
            text: TextOptions::default(),
            selectable: false,
            enabled: true,
            underline: Underline::Hover,
            link_style: HyperlinkStyle::default(),
            tooltip: None,
            menu: true,
            copy_button: false,
            copy_urls: false,
            open: None,
            link_is_block: false,
        }
    }
}

/// A link of a block as it behaved during this pass.
#[derive(Clone, Debug)]
pub struct LinkReport {
    pub index: usize,
    pub id: Id,
    pub url: Option<String>,
    pub response: Response,
    /// The one activation of this pass: a click or Enter/Space, or a middle or Ctrl click.
    pub activation: Option<LinkActivation>,
}

pub(crate) struct BlockOutput {
    pub response: Response,
    pub links: Vec<LinkReport>,
    pub copied: bool,
}

impl Block {
    pub(crate) fn show(mut self, ui: &mut Ui<'_>) -> BlockOutput {
        let id = match self.id {
            Some(source) => ui.scope.with((self.kind, source)),
            None => ui.next_id(self.kind),
        };
        let enabled = self.enabled && ui.is_enabled();
        let selectable = self.selectable && enabled;
        let look = self.text.resolve(ui);
        let avail = ui.available_width();
        let wrap = if self.text.wrap && avail.is_finite() {
            avail.max(1.0)
        } else {
            f32::INFINITY
        };
        let mut state = ui.context.selection.items.remove(&id).unwrap_or_else(|| {
            let env = crate::components::text_edit::doc::Env {
                size: look.size,
                font: look.font,
                wrap,
                tab: look.tab,
                lh: look.size * 1.25,
            };
            BlockState::new(&self.source, env)
        });
        state.last_frame = ui.context.frame;
        let size = layout::prepare(ui.context, &mut state, &self, &look, wrap, avail);
        let extra = if self.copy_button {
            copy_button::GAP + copy_button::SIZE
        } else {
            0.0
        };
        let rect = ui.allocate_space(Vec2::new(size.x + extra, size.y.max(look.size)));
        let text_rect = Rect::from_min_size(rect.min, size);
        let origin = text_rect.min;

        let scope = if self.selectable {
            ui.context.selection_scope_id(id)
        } else {
            id
        };
        let ord = if self.selectable {
            let ord = ui.context.selection_enter(scope, id);
            let scoped = ui.context.selection.stack.is_empty();
            if let Some(info) = ui.context.selection.scopes.get_mut(&scope) {
                if scoped {
                    info.copy_urls = self.copy_urls;
                    info.separator = "\n".into();
                }
            }
            ord
        } else {
            0
        };
        state.scope = scope;
        state.ord = ord;

        let mut runs = links::collect(ui, &self, &mut state, id, origin, enabled, selectable);
        if selectable {
            ui.context.register_hit(HitRegion {
                id,
                window: ui.window,
                rect: text_rect,
                clip: ui.clip,
                action: HitAction::StaticText,
            });
        }
        links::interact(ui, &mut runs, selectable);
        let own = selectable.then(|| ui.response(id, text_rect, true));
        if self.selectable {
            ui.context.selection.owners.insert(id, (id, scope));
        }
        let ids: Vec<Id> = runs.iter().map(|l| l.id).collect();
        for link in &ids {
            if self.selectable {
                ui.context.selection.owners.insert(*link, (id, scope));
            }
        }
        if state.links.len() != runs.len()
            || state
                .links
                .iter()
                .zip(&runs)
                .any(|(a, b)| a.id != b.id || a.range != b.range)
        {
            state.links = links_info(&self, &runs);
        }
        let place = select::Place {
            id,
            scope,
            ord,
            rect: text_rect,
        };
        if selectable {
            select::handle(
                ui,
                &mut state,
                place,
                std::iter::once(id).chain(ids.iter().copied()),
            );
            access::requests(ui, &state, place);
            if ui.context.selection.stack.is_empty() {
                ui.context.selection_commit(scope);
            }
        }

        let selected = if selectable {
            let range = ui.context.selection_range(scope, id, ord, state.text.len());
            state.to_display(range)
        } else {
            0..0
        };
        let active = ui.context.selection_active(scope);
        let style = {
            let mut style = ui.style().hyperlink;
            let own = self.link_style;
            style.color = own.color.or(style.color);
            style.hovered = own.hovered.or(style.hovered);
            style.pressed = own.pressed.or(style.pressed);
            style.visited = own.visited.or(style.visited);
            style.disabled = own.disabled.or(style.disabled);
            style.underline = own.underline.or(style.underline);
            style.thickness = own.thickness.or(style.thickness);
            style.focus = own.focus.or(style.focus);
            style
        };
        let colored = layout::runs(&self.spans, &state, Some(&runs));
        state.doc.set_runs(colored);
        let args = paint::Args {
            id,
            rect: text_rect,
            look,
            links: &runs,
            selected,
            active,
            focus_ring: own.is_some_and(|r| r.focus_visible),
            style,
        };
        paint::paint(ui, &mut state, &self, &args);
        access::describe(ui, &mut state, &self, place, selectable, &runs);

        let response = own
            .or_else(|| runs.iter().find_map(|l| l.response))
            .unwrap_or_else(|| ui.response(id, text_rect, false));
        if self.menu && enabled {
            menu::show(ui, &mut state, id, scope, own, &runs, response);
        }
        let mut copied = false;
        if self.copy_button && enabled {
            copied = copy_button::show(ui, &mut state, id, text_rect);
        }
        let mut reports = Vec::with_capacity(runs.len());
        for link in &runs {
            let Some(mut r) = link.response else { continue };
            let url = state.links.get(link.index).and_then(|l| l.url.clone());
            if link.activation.is_some() {
                if let (Some(policy), Some(url)) = (&self.open, &url) {
                    if let Err(error) = policy.open(url) {
                        ui.context.report(
                            crate::DiagnosticKind::External,
                            Some(link.id),
                            Some(link.bounds),
                            || format!("link not opened: {error}"),
                        );
                    }
                }
            }
            r.set_link(link.activation);
            if link.response.is_some_and(|r| r.hovered) && link.enabled {
                let text = self
                    .spans
                    .iter()
                    .filter_map(|s| s.link.as_ref())
                    .nth(link.index)
                    .and_then(|l| l.tooltip.clone())
                    .or_else(|| url.clone());
                if let Some(text) = text.filter(|_| self.tooltip.is_none()) {
                    Tooltip::new(text).show(ui, r);
                }
            }
            reports.push(LinkReport {
                index: link.index,
                id: link.id,
                url,
                response: r,
                activation: link.activation,
            });
        }
        let mut response = if let Some(first) = reports.first().filter(|_| !self.selectable) {
            first.response
        } else {
            response
        };
        if !self.selectable {
            response.rect = text_rect;
        }
        if let Some(tip) = self.tooltip.take() {
            Tooltip::new(tip).show(ui, response);
        }
        ui.context.selection.items.insert(id, state);
        BlockOutput {
            response,
            links: reports,
            copied,
        }
    }
}

fn links_info(block: &Block, runs: &[links::LinkRun]) -> Vec<state::LinkInfo> {
    let spans = block.spans.iter().filter_map(|s| s.link.as_ref());
    spans
        .zip(runs)
        .map(|(link, run)| state::LinkInfo {
            id: run.id,
            range: run.range.clone(),
            url: link.url.clone(),
        })
        .collect()
}
