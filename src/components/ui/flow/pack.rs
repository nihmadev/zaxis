//! Greedy line breaking for `Wrap`: arithmetic on measured sizes, in declaration order.
//!
//! An item joins the current line when the line, the gap and the item still fit in the
//! width; otherwise it starts the next one. The first item of a line always joins it, so a
//! line is never empty and an item wider than the width simply gets a line of its own.
use super::Measure;
use crate::Vec2;

/// Largest extent taken from a measured item or a builder value. Keeps the sums of a
/// pathological scene finite; real layouts are far below it.
pub(crate) const MAX_EXTENT: f32 = 1.0e6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Line {
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct Slot {
    pub line: usize,
    /// Offset in the line before the line is aligned.
    pub x: f32,
}

pub(super) struct Packed {
    pub slots: Vec<Slot>,
    pub lines: Vec<Line>,
}

/// A measured extent made finite and non-negative.
pub(super) fn clean(size: Vec2) -> Vec2 {
    let part = |v: f32| {
        if v.is_finite() {
            v.clamp(0.0, MAX_EXTENT)
        } else {
            0.0
        }
    };
    Vec2::new(part(size.x), part(size.y))
}

/// Room by which a line may exceed the width, so that sums of fractional sizes that land
/// exactly on the boundary do not wrap because of rounding.
fn tolerance(width: f32) -> f32 {
    1.0e-3_f32.max(width * 2.0 * f32::EPSILON)
}

/// Lines and per-item slots. `measures` must be [`clean`]ed; `width` and `gap` finite and
/// non-negative.
///
/// * a gap item (`Ui::add_space`) adds its width between two items of one line, and
///   vanishes at the start or end of a line;
/// * an item without extent takes no room and no gap, and stays where the next one would go;
/// * `gap.x` separates items of a line, `gap.y` separates lines.
pub(super) fn pack(measures: &[Measure], width: f32, gap: Vec2) -> Packed {
    let tolerance = tolerance(width);
    let mut slots = vec![Slot::default(); measures.len()];
    let mut lines: Vec<Line> = Vec::new();
    let mut line = Line::default();
    let mut count = 0;
    let mut pending = 0.0;
    let mut next_y = 0.0;
    for (slot, measure) in slots.iter_mut().zip(measures) {
        let size = measure.size;
        if measure.gap {
            pending += size.x;
        } else if size == Vec2::ZERO {
            *slot = Slot {
                line: lines.len(),
                x: line.width,
            };
        } else {
            let mut lead = if count == 0 { 0.0 } else { pending + gap.x };
            if count > 0 && line.width + lead + size.x > width + tolerance {
                line.y = next_y;
                next_y += line.height + gap.y;
                lines.push(line);
                line = Line::default();
                count = 0;
                lead = 0.0;
            }
            *slot = Slot {
                line: lines.len(),
                x: line.width + lead,
            };
            line.width += lead + size.x;
            line.height = line.height.max(size.y);
            count += 1;
            pending = 0.0;
        }
        if measure.gap {
            *slot = Slot {
                line: lines.len(),
                x: line.width,
            };
        }
    }
    if count > 0 {
        line.y = next_y;
        lines.push(line);
    }
    Packed { slots, lines }
}
