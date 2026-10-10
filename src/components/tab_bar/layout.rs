//! Tab widths: content measurement, evenly shrinking rows, ellipsis and pixel snapping.

use super::{options::TabWidth, style::Look};
use crate::{
    components::segmented::layout::{fit_label, snap_widths, Label},
    Context, Vec2,
};

pub(super) struct Item {
    pub text: String,
    pub has_icon: bool,
    pub closable: bool,
}

pub(super) struct Plan {
    /// Tab widths in logical pixels, whole physical pixels each.
    pub widths: Vec<f32>,
    /// What is drawn per tab; `None` when it has no label or no room for one.
    pub labels: Vec<Option<Label>>,
    /// The tabs do not fit even at their minimum: the row scrolls.
    pub overflow: bool,
    pub total: f32,
}

fn measure(ctx: &mut Context, text: &str, look: &Look) -> Vec2 {
    if text.is_empty() {
        Vec2::ZERO
    } else {
        ctx.measure_text(text, look.font, look.weight_selected, f32::INFINITY)
    }
}

/// What a tab needs apart from its label: padding, icon and the room kept for the close
/// button whether or not it shows, so that its appearing never moves anything.
fn chrome(item: &Item, text: f32, look: &Look) -> f32 {
    let icon = if item.has_icon {
        look.icon + if text > 0.0 { look.icon_gap } else { 0.0 }
    } else {
        0.0
    };
    let close = if item.closable {
        look.close_reserve()
            + if text > 0.0 || item.has_icon {
                look.close_gap
            } else {
                0.0
            }
    } else {
        0.0
    };
    2.0 * look.pad_x + icon + close
}

/// Widths from a common ceiling down: the widest tabs give way first, none below its floor.
fn water_fill(wanted: &[f32], floors: &[f32], available: f32) -> Option<Vec<f32>> {
    let at = |level: f32| -> Vec<f32> {
        wanted
            .iter()
            .zip(floors)
            .map(|(w, f)| w.min(level).max(*f))
            .collect()
    };
    if floors.iter().sum::<f32>() > available {
        return None;
    }
    let (mut lo, mut hi) = (0.0_f32, wanted.iter().copied().fold(0.0, f32::max));
    for _ in 0..40 {
        let mid = (lo + hi) * 0.5;
        if at(mid).iter().sum::<f32>() > available {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some(at(lo))
}

pub(super) fn plan(
    ctx: &mut Context,
    items: &[Item],
    look: &Look,
    width: TabWidth,
    available: f32,
) -> Plan {
    let available = if available.is_finite() {
        available.max(0.0)
    } else {
        f32::MAX
    };
    let texts: Vec<Vec2> = items.iter().map(|i| measure(ctx, &i.text, look)).collect();
    let content: Vec<f32> = items
        .iter()
        .zip(&texts)
        .map(|(item, text)| chrome(item, text.x, look) + text.x)
        .collect();
    // A tab with a label keeps at least the minimum width; one of an icon alone needs no more.
    let floors: Vec<f32> = items
        .iter()
        .zip(&content)
        .map(|(item, content)| {
            if item.text.is_empty() {
                *content
            } else {
                content.min(look.min_width)
            }
        })
        .collect();
    let bounded: Vec<f32> = items
        .iter()
        .zip(&content)
        .map(|(item, content)| {
            if item.text.is_empty() {
                *content
            } else {
                content.clamp(look.min_width, look.max_width)
            }
        })
        .collect();
    let n = items.len().max(1) as f32;
    let mut wanted = match width {
        TabWidth::Content => bounded,
        TabWidth::Fixed(w) => vec![w.max(1.0); items.len()],
        TabWidth::Equal => {
            let share = available / n;
            let total: f32 = content.iter().sum();
            if content.iter().all(|c| *c <= share) {
                vec![share; items.len()]
            } else if total <= available {
                let extra = (available - total) / n;
                content.iter().map(|c| c + extra).collect()
            } else {
                content.clone()
            }
        }
    };
    let floors: Vec<f32> = match width {
        TabWidth::Fixed(w) => floors.iter().map(|f| f.min(w.max(1.0))).collect(),
        _ => floors,
    };
    for (w, f) in wanted.iter_mut().zip(&floors) {
        *w = w.max(*f);
    }
    let sum: f32 = wanted.iter().sum();
    let (mut widths, overflow, forced) = if sum <= available + 0.001 {
        // A row that fills its container ends exactly at its edge.
        let fills = matches!(width, TabWidth::Equal) && (sum - available).abs() < 0.01;
        (wanted, false, fills)
    } else {
        match water_fill(&wanted, &floors, available) {
            Some(widths) => (widths, false, true),
            None => (floors, true, false),
        }
    };
    widths = snap_widths(&widths, forced.then_some(available), look.scale);
    let labels = items
        .iter()
        .zip(&widths)
        .zip(&texts)
        .map(|((item, width), text)| {
            if text.x <= 0.0 {
                return None;
            }
            let budget = (width - chrome(item, text.x, look)).max(0.0);
            if text.x <= budget + 0.01 {
                Some(Label {
                    text: item.text.clone(),
                    size: *text,
                    truncated: false,
                })
            } else {
                fit_label(ctx, &item.text, look.font, look.weight_selected, budget)
            }
        })
        .collect();
    let total = widths.iter().sum();
    Plan {
        widths,
        labels,
        overflow,
        total,
    }
}
