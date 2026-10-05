//! Page indicators: a pill that flows between pale dots, equal dots, strokes, or a count.
use super::{options::CarouselIndicator, style::Look};
use crate::{
    components::{Sense, Ui},
    context::Paint,
    Border, Color, CornerRadius, Id, Interpolate, Rect, Shape, Vec2,
};

/// Beyond this many pages the items would not fit: show the count instead.
pub(super) const MAX_ITEMS: usize = 24;
/// Smallest pointer target of an item.
const HIT: f32 = 18.0;

pub(super) fn resolve(
    kind: Option<CarouselIndicator>,
    default: CarouselIndicator,
    n: usize,
) -> CarouselIndicator {
    match kind.unwrap_or(default) {
        CarouselIndicator::None => CarouselIndicator::None,
        _ if n < 2 => CarouselIndicator::None,
        CarouselIndicator::Count => CarouselIndicator::Count,
        _ if n > MAX_ITEMS => CarouselIndicator::Count,
        kind => kind,
    }
}

/// Thickness of a row (or column) reserved for the indicator outside the pages.
pub(super) fn thickness(kind: CarouselIndicator, look: &Look, font: f32) -> f32 {
    match kind {
        CarouselIndicator::None => 0.0,
        CarouselIndicator::Count => font + 4.0,
        _ => look
            .indicator_size()
            .max(HIT.min(look.indicator_size() + 14.0)),
    }
}

/// Closeness of item `i` to the position, 1 on it and 0 a page or more away.
fn weight(i: usize, position: f32, n: usize, wrap: bool) -> f32 {
    let mut d = (i as f32 - position).abs();
    if wrap {
        let n = n as f32;
        let p = position.rem_euclid(n);
        let d0 = (i as f32 - p).abs();
        d = d0.min(n - d0);
    }
    (1.0 - d).clamp(0.0, 1.0)
}

pub(super) struct Row<'a> {
    pub(super) id: Id,
    pub(super) kind: CarouselIndicator,
    pub(super) rect: Rect,
    pub(super) axis: usize,
    pub(super) position: f32,
    pub(super) count: usize,
    pub(super) wrap: bool,
    pub(super) look: &'a Look,
    pub(super) over_media: bool,
    /// The carousel takes input: its items are not disabled for assistive technology.
    pub(super) enabled: bool,
}

/// Paint the indicator inside `rect`, centered; returns the page whose item was clicked.
pub(super) fn show(ui: &mut Ui<'_>, row: &Row<'_>) -> Option<usize> {
    let style = ui.style().clone();
    let look = row.look;
    if row.kind == CarouselIndicator::Count {
        let page = row.position.round().rem_euclid(row.count as f32) as usize + 1;
        let text = format!("{page} / {}", row.count);
        let (size, weight) = (style.font_size, style.typography.weights.control);
        let measured = ui.context.measure_text(&text, size, weight, f32::INFINITY);
        let offset = ui.context.centered_line_offset(&text, size, weight);
        let color = if row.over_media {
            Color::rgba(255, 255, 255, 220)
        } else {
            style.muted_text
        };
        let bounds = crate::Rect::from_min_size(row.rect.center() - measured * 0.5, measured);
        ui.a11y(
            row.id.with("count"),
            bounds,
            crate::AccessRole::Label,
            |node| {
                node.value(text.as_str());
            },
        );
        ui.context.paint(
            row.id.with("count"),
            ui.window,
            ui.clip,
            vec![Paint::Text {
                text,
                position: row.rect.center() - measured * 0.5 + Vec2::new(0.0, offset),
                size,
                weight,
                wrap_width: f32::INFINITY,
                color,
            }],
        );
        return None;
    }
    let (thin, long) = (look.indicator_size(), look.indicator_length());
    let (idle_len, active_len, depth) = match row.kind {
        CarouselIndicator::Pill => (thin, long, thin),
        CarouselIndicator::Dashes => (long * 0.55, long, (thin * 0.5).max(2.0)),
        _ => (thin, thin, thin),
    };
    let (idle, active) = if row.over_media {
        (
            Color::rgba(255, 255, 255, 90),
            Color::rgba(255, 255, 255, 255),
        )
    } else {
        (
            style.muted_text.with_opacity(0.3),
            look.style.indicator_active.unwrap_or(style.accent),
        )
    };
    let idle = look.style.indicator_color.unwrap_or(idle);
    let gap = look.indicator_gap();
    let weights: Vec<f32> = (0..row.count)
        .map(|i| weight(i, row.position, row.count, row.wrap))
        .collect();
    let lengths: Vec<f32> = weights
        .iter()
        .map(|w| idle_len + (active_len - idle_len) * w)
        .collect();
    let total: f32 = lengths.iter().sum::<f32>() + gap * (row.count - 1) as f32;
    let main = row.axis;
    let mut cursor = row.rect.center()[main] - total * 0.5;
    let mut clicked = None;
    let mut paint = Vec::new();
    for (i, (&len, &w)) in lengths.iter().zip(&weights).enumerate() {
        let mut min = Vec2::ZERO;
        let mut size = Vec2::ZERO;
        min[main] = cursor;
        size[main] = len;
        min[1 - main] = row.rect.center()[1 - main] - depth * 0.5;
        size[1 - main] = depth;
        let item = Rect::from_min_size(min, size);
        let mut hit = item;
        // Neighbouring targets share their border instead of overlapping, so a press always
        // belongs to exactly the item under the pointer.
        hit.min[main] -= gap * 0.5;
        hit.max[main] += gap * 0.5;
        hit.min[1 - main] = row.rect.center()[1 - main] - HIT * 0.5;
        hit.max[1 - main] = row.rect.center()[1 - main] + HIT * 0.5;
        let response = ui.interact(hit, (row.id, "item", i), Sense::CLICK);
        if response.clicked() {
            clicked = Some(i);
        }
        ui.a11y(response.id, hit, crate::AccessRole::Button, |node| {
            node.label(format!("Page {}", i + 1))
                .disabled(!row.enabled)
                .clicks(response.id);
        });
        let color = idle.interpolate(&active, w);
        paint.push(Paint::Shape(Shape::Rect {
            rect: item,
            fill: color,
            rounding: CornerRadius::all(depth * 0.5),
            border: Border::NONE,
        }));
        cursor += len + gap;
    }
    ui.context
        .paint(row.id.with("items"), ui.window, ui.clip, paint);
    clicked
}
