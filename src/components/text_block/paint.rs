//! Painting of a block: selection backing, text, link underlines and focus rings. Backing and
//! decorations are vector shapes laid on the physical pixel grid; the text is painted from the
//! layout that hit testing reads, and only paragraphs inside the visible clip are emitted.

use super::{
    links::{fragments, Frag, LinkRun},
    options::Look,
    state::BlockState,
    Block,
};
use crate::{
    components::{
        appearance::alpha,
        hyperlink::HyperlinkStyle,
        text_geometry::{snap, snap_rect},
        Ui,
    },
    context::Paint,
    Border, Color, CornerRadius, Id, Rect, Shape, Vec2,
};
use std::ops::Range;

pub(crate) struct Args<'a> {
    pub id: Id,
    pub rect: Rect,
    pub look: Look,
    pub links: &'a [LinkRun],
    /// Selected bytes of the shown text.
    pub selected: Range<usize>,
    pub active: bool,
    pub focus_ring: bool,
    pub style: HyperlinkStyle,
}

fn line(from: Vec2, to: Vec2, width: f32, color: Color) -> Paint {
    Paint::Shape(Shape::Line {
        start: from,
        end: to,
        width,
        color,
    })
}

pub(crate) fn paint(ui: &mut Ui<'_>, state: &mut BlockState, block: &Block, a: &Args<'_>) {
    let origin = a.rect.min;
    let scale = ui.context.pixel_scale();
    let (window, clip) = (ui.window, ui.clip);
    let visible = ui.clip_rect();
    let opacity = ui.style().opacity;
    let (top, bottom) = (visible.min.y - origin.y, visible.max.y - origin.y);
    let (first, last) = {
        let doc = &mut state.doc;
        (doc.para_at_y(top.max(0.0)), doc.para_at_y(bottom.max(0.0)))
    };

    if !a.selected.is_empty() {
        let style = ui.style();
        let mut fill = style
            .text_edit
            .selection
            .unwrap_or(style.text_edit_selection);
        if !a.active {
            fill.0[3] = (f32::from(fill.0[3]) * 0.5) as u8;
        }
        let shapes = {
            let BlockState {
                doc, text, display, ..
            } = &mut *state;
            let shown: &str = display.as_deref().unwrap_or(text);
            doc.selection_rects(
                ui.context,
                shown,
                a.selected.clone(),
                first..last + 1,
                a.look.size * 0.4,
            )
        };
        let shapes: Vec<Paint> = shapes
            .into_iter()
            .map(|r| snap_rect(r.translate(origin), scale))
            .filter(|r| !r.is_empty())
            .map(|r| Paint::Shape(Shape::rect(r, fill).into()))
            .collect();
        ui.context
            .paint(a.id.with("selection"), window, clip, shapes);
    }

    let color = alpha(a.look.color, opacity);
    let texts: Vec<Paint> = {
        let BlockState {
            doc, text, display, ..
        } = &mut *state;
        let shown: &str = display.as_deref().unwrap_or(text);
        let mut out = Vec::new();
        for i in first..=last.min(doc.paras.len() - 1) {
            if !doc.text_of(shown, i).is_empty() {
                let position = origin + Vec2::new(0.0, doc.top(i));
                out.push(doc.paint(ui.context, shown, i, position, color));
            }
        }
        out
    };
    ui.context.paint(a.id.with("text"), window, clip, texts);

    let thickness = a.style.thickness_for(a.look.size, scale);
    let underline = |frag: &Frag, color: Color| {
        let edge = snap(frag.baseline + (a.look.size * 0.1).max(1.0), scale);
        let y = edge + thickness * 0.5;
        let (x0, x1) = (snap(frag.rect.min.x, scale), snap(frag.rect.max.x, scale));
        line(Vec2::new(x0, y), Vec2::new(x1, y), thickness, color)
    };
    let mut deco = Vec::new();
    let ring = ui
        .style()
        .hyperlink
        .focus
        .unwrap_or(ui.style().focus_border);
    for link in a.links {
        if link.underline_alpha > 0.0 {
            let color = alpha(link.color, link.underline_alpha * opacity);
            for frag in link
                .frags
                .iter()
                .filter(|f| f.rect.intersect(visible).size().x > 0.0)
            {
                deco.push(underline(frag, color));
            }
        }
        if link.response.is_some_and(|r| r.focus_visible) {
            for frag in &link.frags {
                deco.push(Paint::Shape(Shape::Rect {
                    rect: Rect::from_min_max(
                        frag.rect.min - Vec2::splat(2.0),
                        frag.rect.max + Vec2::splat(2.0),
                    ),
                    fill: Color::TRANSPARENT,
                    rounding: CornerRadius::all(3.0),
                    border: ring,
                }));
            }
        }
    }
    for span in block
        .spans
        .iter()
        .filter(|s| s.link.is_none() && s.style.underline)
    {
        let range = state.to_display(span.range.clone());
        let frags = fragments(ui, state, range, origin);
        let color = alpha(span.style.color.unwrap_or(a.look.color), opacity);
        deco.extend(frags.iter().map(|f| underline(f, color)));
    }
    if a.focus_ring {
        let color = Color(ring.color.0).with_opacity(0.55);
        deco.push(Paint::Shape(Shape::Rect {
            rect: Rect::from_min_max(a.rect.min - Vec2::splat(3.0), a.rect.max + Vec2::splat(3.0)),
            fill: Color::TRANSPARENT,
            rounding: CornerRadius::all(4.0),
            border: Border::new(1.0, color),
        }));
    }
    ui.context.paint(a.id.with("deco"), window, clip, deco);
}
