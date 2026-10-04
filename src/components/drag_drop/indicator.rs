//! Drop indicators: target highlight, rejection outline and insertion line.
//! They are painted from the target's own bounds in the target's window, so
//! they move, clip and scale together with the content.
use super::{style::DragStyle, DropZones, Insertion};
use crate::{
    context::{drag::state::Hover, Paint},
    Color, CornerRadius, Id, Layout, Rect, Shape, Ui, Vec2,
};

pub(super) struct Indicator {
    pub id: Id,
    pub rect: Rect,
    pub hover: Hover,
    pub zones: Option<DropZones>,
    /// Space kept free at the line's start, for example a tree level indent.
    pub indent: f32,
}

/// Fade-in progress shared by all targets, so moving between rows keeps the
/// indicator at full strength instead of restarting a fade per row.
fn alpha_id() -> Id {
    crate::context::drag::preview::layer_id().with("indicator")
}

pub(super) fn paint(ui: &mut Ui<'_>, style: &DragStyle, indicator: Indicator) {
    let Indicator {
        id,
        rect,
        hover,
        zones,
        indent,
    } = indicator;
    if rect.is_empty() {
        return;
    }
    let tween = style.tween(ui.style());
    let visible = !rect.intersect(ui.clip_rect()).is_empty();
    let alpha = ui
        .context
        .transition_visible(alpha_id(), Some(0.0_f32), 1.0_f32, tween, visible)
        .value
        .clamp(0.0, 1.0);
    let paints = style.paints(ui.style());
    let fade = |mut color: Color| {
        color.0[3] = (f32::from(color.0[3]) * alpha).round() as u8;
        color
    };
    let mut shapes = Vec::new();
    match (hover.accepts, hover.insertion) {
        (false, _) => {
            let mut border = paints.reject;
            border.color = fade(border.color);
            shapes.push(outline(rect, paints.rounding, border));
        }
        (true, Some(zone @ (Insertion::Before | Insertion::After))) => {
            let layout = zones.map_or(Layout::Vertical, |z| z.layout);
            shapes.extend(line(
                rect,
                Line {
                    zone,
                    layout,
                    indent,
                    thickness: paints.thickness,
                    color: fade(paints.line),
                    band: fade(paints.fill),
                    rounding: paints.rounding,
                },
            ));
        }
        (true, _) => {
            shapes.push(
                Shape::rect(rect, fade(paints.fill))
                    .corner_radius(paints.rounding)
                    .border(crate::Border::new(
                        paints.border.width,
                        fade(paints.border.color),
                    ))
                    .into(),
            );
        }
    }
    ui.context.paint(
        id.with("drag-indicator"),
        ui.window,
        ui.clip,
        shapes.into_iter().map(Paint::Shape).collect(),
    );
}

fn outline(rect: Rect, rounding: CornerRadius, border: crate::Border) -> Shape {
    Shape::rect(rect, Color::TRANSPARENT)
        .corner_radius(rounding)
        .border(border)
        .into()
}

struct Line {
    zone: Insertion,
    layout: Layout,
    indent: f32,
    thickness: f32,
    color: Color,
    band: Color,
    rounding: CornerRadius,
}

/// A translucent band over the targeted element and a rounded line on the
/// insertion edge, kept inside the element so row clips never cut it.
fn line(rect: Rect, l: Line) -> Vec<Shape> {
    let t = l.thickness.max(1.0);
    let mut soft = l.band;
    soft.0[3] /= 2;
    let strip = match l.layout {
        Layout::Vertical => {
            let y = if l.zone == Insertion::Before {
                rect.min.y
            } else {
                rect.max.y - t
            };
            let start = (rect.min.x + l.indent).min(rect.max.x);
            Rect::from_min_size(Vec2::new(start, y), Vec2::new(rect.max.x - start, t))
        }
        Layout::Horizontal => {
            let x = if l.zone == Insertion::Before {
                rect.min.x
            } else {
                rect.max.x - t
            };
            let start = (rect.min.y + l.indent).min(rect.max.y);
            Rect::from_min_size(Vec2::new(x, start), Vec2::new(t, rect.max.y - start))
        }
    };
    vec![
        Shape::rect(rect, soft).corner_radius(l.rounding).into(),
        Shape::rect(strip, l.color)
            .corner_radius(CornerRadius::all(t * 0.5))
            .into(),
    ]
}
