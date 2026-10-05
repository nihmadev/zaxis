//! Segment sizes: content measurement, shrinking, truncation and pixel snapping.

use unicode_segmentation::UnicodeSegmentation;

use super::options::SegmentWidth;
use crate::{Context, FontWeight, Vec2};

pub struct Metrics {
    pub font: f32,
    pub weight: FontWeight,
    pub pad_x: f32,
    pub icon: f32,
    pub icon_gap: f32,
    pub seg_h: f32,
    pub padding: f32,
    pub gap: f32,
    pub scale: f32,
}

pub struct Item {
    pub text: String,
    pub has_icon: bool,
}

pub struct Request {
    pub width: SegmentWidth,
    pub min_width: f32,
    pub vertical: bool,
    pub icon_only: bool,
    /// Room for a selection mark in segments without an icon, so selecting never reflows.
    pub reserve_mark: bool,
    pub available: f32,
}

pub struct Label {
    pub text: String,
    pub size: Vec2,
    pub truncated: bool,
}

pub struct Plan {
    /// Segment widths in logical pixels, each a whole number of physical pixels.
    pub widths: Vec<f32>,
    /// What is drawn per segment; `None` when the label is hidden.
    pub labels: Vec<Option<Label>>,
    /// The whole control, plate padding included.
    pub size: Vec2,
}

pub fn snap(value: f32, scale: f32) -> f32 {
    (value * scale).round() / scale
}

/// Rounds to physical pixels so the parts add up to the rounded whole (largest remainder),
/// without drift at any scale factor. `total` forces the sum; `None` rounds each part alone.
fn snap_widths(widths: &[f32], total: Option<f32>, scale: f32) -> Vec<f32> {
    let px: Vec<f32> = widths.iter().map(|w| (w * scale).max(0.0)).collect();
    if total.is_none() {
        return px.iter().map(|v| v.round() / scale).collect();
    }
    let mut out: Vec<f32> = px.iter().map(|v| v.floor()).collect();
    let target = (total.unwrap_or(0.0) * scale).floor();
    let mut left = (target - out.iter().sum::<f32>()).max(0.0) as usize;
    let mut order: Vec<usize> = (0..px.len()).collect();
    order.sort_by(|a, b| (px[*b].fract()).total_cmp(&px[*a].fract()));
    while left > 0 && !order.is_empty() {
        for &i in order.iter().take(left) {
            out[i] += 1.0;
        }
        left -= left.min(order.len());
    }
    out.iter().map(|v| v / scale).collect()
}

fn measure(ctx: &mut Context, text: &str, m: &Metrics) -> Vec2 {
    if text.is_empty() {
        Vec2::ZERO
    } else {
        ctx.measure_text(text, m.font, m.weight, f32::INFINITY)
    }
}

/// The longest prefix on a grapheme boundary that fits `budget` together with an ellipsis.
fn fit_text(ctx: &mut Context, text: &str, m: &Metrics, budget: f32) -> Option<Label> {
    let cuts: Vec<usize> = text.grapheme_indices(true).map(|(i, _)| i).collect();
    let build = |keep: usize| format!("{}…", text[..cuts[keep]].trim_end());
    let (mut lo, mut hi) = (0, cuts.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if measure(ctx, &build(mid), m).x <= budget {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    let shown = build(lo);
    let size = measure(ctx, &shown, m);
    (size.x <= budget).then_some(Label {
        text: shown,
        size,
        truncated: true,
    })
}

pub fn plan(ctx: &mut Context, items: &[Item], m: &Metrics, req: &Request) -> Plan {
    let n = items.len().max(1);
    let texts: Vec<Vec2> = items.iter().map(|i| measure(ctx, &i.text, m)).collect();
    let gaps = m.gap * (items.len().saturating_sub(1)) as f32;
    let available = if req.available.is_finite() {
        req.available.max(0.0)
    } else {
        f32::MAX
    };
    let inner = if req.vertical {
        (available - 2.0 * m.padding).max(0.0)
    } else {
        (available - 2.0 * m.padding - gaps).max(0.0)
    };
    let chrome = |i: usize, hidden: bool| {
        let text = if hidden { 0.0 } else { texts[i].x };
        let icon = if items[i].has_icon || req.reserve_mark {
            m.icon + if text > 0.0 { m.icon_gap } else { 0.0 }
        } else {
            0.0
        };
        2.0 * m.pad_x + icon + text
    };
    // Widths for one choice of hidden labels, and whether they were forced to the container.
    let solve = |hidden: &[bool]| -> (Vec<f32>, bool) {
        let natural: Vec<f32> = (0..items.len()).map(|i| chrome(i, hidden[i])).collect();
        let widest = natural.iter().copied().fold(0.0, f32::max);
        let base: Vec<f32> = match req.width {
            SegmentWidth::Content => natural,
            SegmentWidth::Fixed(f) => vec![f; items.len()],
            _ => vec![widest; items.len()],
        }
        .into_iter()
        .map(|w| w.max(req.min_width))
        .collect();
        let total: f32 = base.iter().sum();
        if req.vertical {
            let column = match req.width {
                SegmentWidth::Fill => inner,
                _ => base.iter().copied().fold(0.0, f32::max).min(inner),
            };
            let forced = column < base.iter().copied().fold(0.0, f32::max);
            (
                vec![column; items.len()],
                forced || req.width == SegmentWidth::Fill,
            )
        } else if req.width == SegmentWidth::Fill {
            (vec![inner / n as f32; items.len()], true)
        } else if total > inner {
            (base.iter().map(|w| w * inner / total).collect(), true)
        } else {
            (base, false)
        }
    };
    let mut hidden: Vec<bool> = items.iter().map(|i| req.icon_only && i.has_icon).collect();
    let (mut widths, mut forced) = solve(&hidden);
    if !req.icon_only {
        let min_label = measure(ctx, "…", m).x + m.font * 0.6;
        let unreadable = items.iter().enumerate().any(|(i, item)| {
            item.has_icon
                && texts[i].x > 0.0
                && widths[i] - 2.0 * m.pad_x - m.icon - m.icon_gap < min_label
        });
        if unreadable {
            for (flag, item) in hidden.iter_mut().zip(items) {
                *flag |= item.has_icon;
            }
            (widths, forced) = solve(&hidden);
        }
    }
    widths = if req.vertical {
        widths
            .iter()
            .map(|w| (w * m.scale).floor().max(0.0) / m.scale)
            .collect()
    } else {
        snap_widths(&widths, forced.then_some(inner), m.scale)
    };
    let labels = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            if hidden[i] || texts[i].x <= 0.0 {
                return None;
            }
            let icon = if item.has_icon || req.reserve_mark {
                m.icon + m.icon_gap
            } else {
                0.0
            };
            let budget = (widths[i] - 2.0 * m.pad_x - icon).max(0.0);
            if texts[i].x <= budget + 0.01 {
                Some(Label {
                    text: item.text.clone(),
                    size: texts[i],
                    truncated: false,
                })
            } else {
                fit_text(ctx, &item.text, m, budget)
            }
        })
        .collect();
    let count = items.len() as f32;
    let size = if req.vertical {
        Vec2::new(
            2.0 * m.padding + widths.first().copied().unwrap_or(0.0),
            2.0 * m.padding + count * m.seg_h + (count - 1.0).max(0.0) * m.gap,
        )
    } else {
        Vec2::new(
            2.0 * m.padding + widths.iter().sum::<f32>() + gaps,
            2.0 * m.padding + m.seg_h,
        )
    };
    Plan {
        widths,
        labels,
        size,
    }
}
