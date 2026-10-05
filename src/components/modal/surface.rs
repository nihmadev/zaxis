//! The modal surface: its pose, chrome and input block, the header, body and
//! actions in one content column, and the corner close button. Every part is
//! measured at its natural size for the next pass's geometry.
use std::ops::Range;

use super::{geometry::Measure, look::Resolved, parts, Ui};
use crate::{
    components::ScrollArea,
    context::{HitAction, HitRegion},
    layout::LayoutCursor,
    CornerRadius, Id, Layout, Rect, Transform, Vec2,
};

pub(super) type Part<'a> = Box<dyn FnOnce(&mut Ui<'_>) + 'a>;

/// What the application builds inside the surface. The body runs exactly once per
/// pass; header and actions stay outside its scrolling area.
pub(super) struct Content<'a, B> {
    pub(super) header: Option<Part<'a>>,
    pub(super) body: B,
    pub(super) footer: Option<Part<'a>>,
}

/// Where and how the surface is built this pass.
pub(super) struct Frame {
    pub(super) id: Id,
    /// The settled surface in screen coordinates.
    pub(super) rect: Rect,
    pub(super) viewport: Rect,
    /// Corners of the surface (edge sheets square their edge side).
    pub(super) rounding: CornerRadius,
    /// Height the scrolling body may take.
    pub(super) body_height: f32,
    /// Entering or leaving pose, and its opacity.
    pub(super) transform: Transform,
    pub(super) opacity: f32,
    /// Only an open modal takes input; a closing one is drawn only.
    pub(super) open: bool,
    pub(super) bodyless: bool,
    /// Size of the corner close button, when it is shown.
    pub(super) close: Option<f32>,
}

/// What building the surface produced.
pub(super) struct Built<R> {
    pub(super) inner: R,
    /// Natural sizes of the parts, for the next pass.
    pub(super) measure: Measure,
    pub(super) close_clicked: bool,
    /// Accessibility nodes the header added; they name the dialog.
    pub(super) titles: Range<usize>,
}

/// Build the surface on `root`, the modal's layer.
pub(super) fn build<R>(
    root: &mut Ui<'_>,
    frame: &Frame,
    look: &Resolved,
    content: Content<'_, impl FnOnce(&mut Ui<'_>) -> R>,
) -> Built<R> {
    let mut measure = Measure {
        parts: u8::from(!frame.bodyless)
            + u8::from(content.header.is_some())
            + u8::from(content.footer.is_some()),
        last_frame: root.context.frame,
        ..Default::default()
    };
    let (rect, viewport, id) = (frame.rect, frame.viewport, frame.id);
    root.layout = LayoutCursor::new(rect, Layout::Vertical, 0.0);
    let (inner, titles, close_clicked) =
        root.visual("surface", frame.transform, frame.opacity, |s| {
            // The effect measures this child: report the surface so it counts as visible.
            s.allocate_space(rect.size());
            parts::paint_surface(s, id, rect, viewport, look.surface, frame.rounding);
            if frame.open {
                s.context.register_hit(HitRegion {
                    id: id.with("block"),
                    window: id,
                    rect,
                    clip: viewport,
                    action: HitAction::Block,
                });
            }
            let reserve = frame.close.map_or(0.0, |size| size + 4.0);
            let (inner, titles) = stack(s, frame, look, reserve, content, &mut measure);
            let close_clicked = frame.close.is_some_and(|size| {
                let corner = Rect::from_min_size(
                    Vec2::new(rect.max.x - 8.0 - size, rect.min.y + 8.0),
                    Vec2::splat(size),
                );
                parts::close_button(s, corner)
            });
            (inner, titles, close_clicked)
        });
    measure.width = measure.width.max(0.0);
    Built {
        inner: inner.expect("modal body runs once per pass"),
        measure,
        close_clicked,
        titles,
    }
}

/// Header, body and actions stacked in the content column. Records their natural
/// sizes in `measure`; returns the body's result and the header's accessibility nodes.
fn stack<R>(
    s: &mut Ui<'_>,
    frame: &Frame,
    look: &Resolved,
    reserve: f32,
    content: Content<'_, impl FnOnce(&mut Ui<'_>) -> R>,
    measure: &mut Measure,
) -> (Option<R>, Range<usize>) {
    let Content {
        header,
        body,
        footer,
    } = content;
    let (scroll, gap) = (look.scroll, look.spacing.gap);
    let content = look.spacing.padding.inset(frame.rect);
    let width = content.size().x;
    // Parts are measured at their natural height; the geometry fits them to the surface.
    // The body's scroll padding hangs outside the content column so that
    // controls line up with the header and the actions.
    let bleed = scroll.padding.left.max(0.0);
    let tall = Rect::from_min_size(
        content.min - Vec2::new(bleed, 0.0),
        Vec2::new(width + 2.0 * bleed, 1.0e5),
    );
    let (mut inner, mut titles) = (None, 0..0);
    parts::region(s, tall, gap, frame.id.with("content"), |c| {
        if let Some(header) = header {
            let start = c.context.a11y_len();
            measure.header =
                parts::measure_item(c, |u| indented(u, bleed, width - reserve, header));
            titles = start..c.context.a11y_len();
        }
        if frame.bodyless {
            inner = Some(body(c));
        } else {
            let area = ScrollArea::vertical()
                .id_source("body")
                .max_height(frame.body_height.max(0.0))
                .show(c, |u| inner = Some(body(u)));
            // The bar gutter is always reserved beside the viewport.
            let gutter = scroll.bar_width.max(0.0) + scroll.bar_margin.max(0.0);
            measure.width = area.content_size.x + gutter;
            measure.body = area.content_size.y + scroll.padding.size().y;
        }
        if let Some(footer) = footer {
            measure.footer = parts::measure_item(c, |u| indented(u, bleed, width, footer));
        }
    });
    (inner, titles)
}

/// A header or action row aligned with the body's content column.
fn indented(ui: &mut Ui<'_>, bleed: f32, width: f32, part: Part<'_>) {
    ui.horizontal(|row| {
        row.add_space(bleed);
        row.with_width(width.max(0.0), |column| {
            column.vertical(|stack| part(stack))
        });
    });
}
