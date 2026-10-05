//! Links of a text block: where each one is on screen (one rectangle per wrapped line
//! fragment), the hit regions and responses of those fragments, and the colors the link
//! takes in each state. Fragments of one link share its [`Id`], so a press on one line and a
//! release on the next is still one click, and Tab stops once per link.

use super::{state::BlockState, Block};
use crate::{
    components::{
        hyperlink::{HyperlinkStyle, LinkActivation},
        rich_text::Underline,
        text_geometry::paragraph_slabs,
        Response, Ui,
    },
    context::{HitAction, HitRegion},
    Color, Id, Rect, Vec2,
};
use std::ops::Range;

pub(crate) struct LinkRun {
    pub index: usize,
    pub id: Id,
    /// Source bytes of the link text.
    pub range: Range<usize>,
    pub enabled: bool,
    pub visited: bool,
    pub underline: Underline,
    pub frags: Vec<Frag>,
    pub bounds: Rect,
    pub response: Option<Response>,
    pub color: Color,
    pub underline_alpha: f32,
    pub activation: Option<LinkActivation>,
}

/// Screen rectangle of one line fragment and the baseline of its line.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frag {
    pub rect: Rect,
    pub baseline: f32,
}

/// Screen fragments of the shown bytes `range`, one per line.
pub(crate) fn fragments(
    ui: &mut Ui<'_>,
    state: &mut BlockState,
    range: Range<usize>,
    origin: Vec2,
) -> Vec<Frag> {
    let mut out = Vec::new();
    if range.is_empty() {
        return out;
    }
    let BlockState {
        doc, text, display, ..
    } = state;
    let shown: &str = display.as_deref().unwrap_or(text);
    let (first, last) = (
        doc.para_at(range.start),
        doc.para_at(range.end.saturating_sub(1)),
    );
    for i in first..=last.min(doc.paras.len() - 1) {
        let (start, len) = (doc.paras[i].start, doc.paras[i].len);
        let layout = doc.layout(ui.context, shown, i);
        let rel = range.start.saturating_sub(start)..(range.end - start).min(len);
        let mut slabs = Vec::new();
        paragraph_slabs(&layout, rel, len, 0.0, doc.top(i), &mut slabs);
        out.extend(slabs.into_iter().map(|(rect, baseline)| Frag {
            rect: rect.translate(origin),
            baseline: baseline + origin.y,
        }));
    }
    out
}

pub(crate) fn union(frags: &[Frag]) -> Rect {
    frags
        .iter()
        .fold(None, |acc: Option<Rect>, f| {
            Some(acc.map_or(f.rect, |a| {
                Rect::from_min_max(a.min.min(f.rect.min), a.max.max(f.rect.max))
            }))
        })
        .unwrap_or_default()
}

/// The links of `spans`, with geometry. `block.link_enabled` already folds in the group.
pub(crate) fn collect(
    ui: &mut Ui<'_>,
    block: &Block,
    state: &mut BlockState,
    id: Id,
    origin: Vec2,
    enabled: bool,
    selectable: bool,
) -> Vec<LinkRun> {
    let mut runs = Vec::new();
    for span in &block.spans {
        let Some(link) = &span.link else { continue };
        let index = runs.len();
        let range = span.range.clone();
        let shown = state.to_display(range.clone());
        // A link entirely inside the hidden part has nothing to point at.
        let visible = state.cut.is_none_or(|cut| range.start < cut);
        let frags = if visible {
            fragments(ui, state, shown, origin)
        } else {
            Vec::new()
        };
        runs.push(LinkRun {
            index,
            id: link.id.unwrap_or_else(|| {
                if block.link_is_block && !selectable {
                    id
                } else {
                    id.with(("link", index))
                }
            }),
            range,
            enabled: enabled && link.enabled,
            visited: link.visited,
            underline: link.underline.unwrap_or(block.underline),
            bounds: union(&frags),
            frags,
            response: None,
            color: Color::TRANSPARENT,
            underline_alpha: 0.0,
            activation: None,
        });
    }
    runs
}

/// Register hit regions, read input and animate colors of every link.
pub(crate) fn interact(ui: &mut Ui<'_>, links: &mut [LinkRun], selectable: bool) {
    let style: HyperlinkStyle = ui.style().hyperlink;
    let colors = style.resolve(ui.style());
    let motion = ui.style().motion.hover.clone();
    for link in links.iter_mut() {
        if link.frags.is_empty() {
            continue;
        }
        let action = if link.enabled {
            Some(HitAction::Link)
        } else if !selectable {
            Some(HitAction::Block)
        } else {
            None
        };
        if let Some(action) = action {
            for (n, frag) in link.frags.iter().enumerate() {
                let hit = HitRegion {
                    id: link.id,
                    window: ui.window,
                    rect: frag.rect,
                    clip: ui.clip,
                    action,
                };
                if n == 0 {
                    ui.context.register_hit(hit);
                } else {
                    ui.context.route_hit(hit);
                }
            }
        }
        let mut response = ui.response(link.id, link.bounds, link.enabled);
        response.hovered = link.enabled && ui.context.link_hovered(link.id);
        let ctrl = ui.context.input().modifiers.control_key();
        // A drag that selected text, even one that ends over the link, is not a click.
        let selected_by_drag = ui.context.selection_dragged_from(link.id);
        if link.enabled && !selected_by_drag {
            let middle = ui.context.gestures.middle_clicked(link.id);
            link.activation = if middle {
                Some(LinkActivation::Secondary)
            } else if response.clicked() {
                Some(if ctrl {
                    LinkActivation::Secondary
                } else {
                    LinkActivation::Primary
                })
            } else {
                None
            };
        }
        let target = if !link.enabled {
            colors.disabled
        } else if response.pressed {
            colors.pressed
        } else if response.hovered {
            colors.hovered
        } else if link.visited {
            colors.visited
        } else {
            colors.normal
        };
        link.color = ui
            .transition(("link-color", link.id), target, motion.clone())
            .value;
        let shown = match link.underline {
            Underline::Always => true,
            Underline::Never => false,
            Underline::Hover => link.enabled && (response.hovered || response.focus_visible),
        };
        link.underline_alpha = ui
            .transition(
                ("link-underline", link.id),
                if shown { 1.0_f32 } else { 0.0 },
                motion.clone(),
            )
            .value;
        link.response = Some(response);
    }
}
