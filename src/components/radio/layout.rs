//! Row sizes: text measurement, wrapping, column fitting and pixel snapping.

use crate::{Context, FontWeight, Rect, Vec2};

pub(super) struct Metrics {
    pub font: f32,
    pub desc_font: f32,
    pub weight: FontWeight,
    /// Indicator diameter, a whole number of physical pixels.
    pub d: f32,
    /// Room left of (and around) the indicator for the halo, so it is never cut off by the
    /// group edge; part of the hit area.
    pub inset: f32,
    pub label_gap: f32,
    pub row_gap: f32,
    pub column_gap: f32,
    pub min_height: f32,
    /// Space between the label and its description.
    pub line_gap: f32,
    pub scale: f32,
}

pub(super) struct Item {
    pub label: String,
    pub description: String,
}

/// One option's row, relative to the group's top-left corner.
pub(super) struct Cell {
    /// The whole row: indicator, gap and text column; this is also the hit area.
    pub rect: Rect,
    /// Width the label and the description wrap at.
    pub text_w: f32,
    pub label_h: f32,
    pub desc_h: f32,
    /// Top of the text block inside the row.
    pub text_top: f32,
    /// Height of one label line; the indicator centers on it.
    pub line_h: f32,
}

pub(super) struct Plan {
    pub cells: Vec<Cell>,
    pub size: Vec2,
}

pub(super) fn snap(value: f32, scale: f32) -> f32 {
    (value * scale).round() / scale
}

fn floor_px(value: f32, scale: f32) -> f32 {
    (value * scale + 1e-3).floor() / scale
}

fn ceil_px(value: f32, scale: f32) -> f32 {
    (value * scale - 1e-3).ceil() / scale
}

fn measure(ctx: &mut Context, text: &str, size: f32, m: &Metrics, wrap: f32) -> Vec2 {
    if text.is_empty() {
        Vec2::ZERO
    } else {
        ctx.measure_text(text, size, m.weight, wrap)
    }
}

/// Column widths for `natural` widths inside `budget`: narrow columns keep their natural
/// width, the wide ones share what is left, none gets below `min`.
fn fit_columns(natural: &[f32], budget: f32, min: f32) -> Vec<f32> {
    let mut order: Vec<usize> = (0..natural.len()).collect();
    order.sort_by(|a, b| natural[*a].total_cmp(&natural[*b]));
    let mut widths = natural.to_vec();
    let mut left = budget;
    for (rank, &i) in order.iter().enumerate() {
        let share = left / (order.len() - rank) as f32;
        widths[i] = natural[i].min(share).max(min.min(natural[i]));
        left -= widths[i];
    }
    widths
}

pub(super) fn plan(
    ctx: &mut Context,
    items: &[Item],
    m: &Metrics,
    cols: usize,
    available: f32,
) -> Plan {
    let n = items.len();
    let cols = cols.clamp(1, n.max(1));
    let line_h = measure(ctx, "Ag", m.font, m, f32::INFINITY).y;
    let natural_text: Vec<f32> = items
        .iter()
        .map(|i| {
            let label = measure(ctx, &i.label, m.font, m, f32::INFINITY).x;
            let desc = measure(ctx, &i.description, m.desc_font, m, f32::INFINITY).x;
            ceil_px(label.max(desc), m.scale)
        })
        .collect();
    let lead = |text: f32| m.inset + m.d + if text > 0.0 { m.label_gap } else { 0.0 };
    let mut natural = vec![0.0_f32; cols];
    for (i, text) in natural_text.iter().enumerate() {
        natural[i % cols] = natural[i % cols].max(lead(*text) + text);
    }
    let gaps = m.column_gap * (cols - 1) as f32;
    let mut widths = natural.clone();
    if available.is_finite() && natural.iter().sum::<f32>() + gaps > available {
        let min = m.inset + m.d + m.label_gap + m.font * 2.0;
        widths = fit_columns(&natural, (available - gaps).max(0.0), min);
    }
    let widths: Vec<f32> = widths.iter().map(|w| floor_px(*w, m.scale)).collect();

    let rows = n.div_ceil(cols);
    let mut cells: Vec<Cell> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let col_w = widths[i % cols];
            // One logical pixel of slack keeps a label that exactly fits on one line.
            let text_w = if natural_text[i] > 0.0 {
                (col_w - m.inset - m.d - m.label_gap).max(1.0) + 1.0
            } else {
                0.0
            };
            Cell {
                rect: Rect::from_min_size(Vec2::ZERO, Vec2::new(col_w, 0.0)),
                text_w,
                label_h: measure(ctx, &item.label, m.font, m, text_w).y,
                desc_h: measure(ctx, &item.description, m.desc_font, m, text_w).y,
                text_top: 0.0,
                line_h,
            }
        })
        .collect();
    let block = |c: &Cell| {
        c.label_h
            + if c.desc_h > 0.0 {
                m.line_gap + c.desc_h
            } else {
                0.0
            }
    };
    let mut y = 0.0;
    for r in 0..rows {
        let row = r * cols..((r + 1) * cols).min(n);
        let height = row
            .clone()
            .map(|i| block(&cells[i]))
            .fold(m.min_height, f32::max);
        let height = ceil_px(height.max(m.d + 2.0 * m.inset), m.scale);
        let mut x = 0.0;
        for i in row {
            let cell = &mut cells[i];
            let top = snap((height - block(cell)) * 0.5, m.scale).max(0.0);
            cell.text_top = top;
            cell.rect = Rect::from_min_size(Vec2::new(x, y), Vec2::new(cell.rect.size().x, height));
            x += cell.rect.size().x + m.column_gap;
        }
        y += height + m.row_gap;
    }
    let size = Vec2::new(widths.iter().sum::<f32>() + gaps, (y - m.row_gap).max(0.0));
    Plan { cells, size }
}
