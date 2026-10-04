//! Scroll bookkeeping of a multi-line field. The offset itself lives in the standard
//! `ScrollArea` state; this module only decides when to move it: keeping the top visual
//! line steady when heights or wrapping change, and scrolling while a drag selects.
use super::{doc::Doc, *};
use crate::{components::edit_buffer::Delta, context::Context};
use std::time::Instant;

/// The visual line at the top of the viewport, tracked by byte so that edits above it,
/// re-wrapping and height corrections keep the same text in place.
pub(super) struct Anchor {
    byte: usize,
    /// Distance from the top of that visual line to the viewport top.
    intra: f32,
}

impl Anchor {
    pub fn capture(doc: &mut Doc, ctx: &mut Context, text: &str, offset_y: f32) -> Self {
        let i = doc.para_at_y(offset_y);
        let into = offset_y - doc.top(i);
        let layout = doc.layout(ctx, text, i);
        let lh = doc.env().lh;
        let line = ((into / lh).floor().max(0.0) as usize).min(layout.lines.len() - 1);
        Self {
            byte: doc.paras[i].start + layout.lines[line].start,
            intra: into - line as f32 * lh,
        }
    }

    pub fn shift(&mut self, delta: Delta) {
        if delta.at + delta.old_len <= self.byte {
            self.byte = self.byte + delta.new_len - delta.old_len;
        } else if delta.at < self.byte {
            self.byte = delta.at;
        }
    }

    /// The scroll offset that puts the tracked line back at the top.
    pub fn resolve(&self, doc: &mut Doc, ctx: &mut Context, text: &str) -> f32 {
        let i = doc.para_at(self.byte.min(text.len()));
        let layout = doc.layout(ctx, text, i);
        let rel = self.byte.saturating_sub(doc.paras[i].start);
        let line = Doc::line_of(&layout.lines, rel, false);
        doc.top(i) + layout.lines[line].top + self.intra
    }
}

const SPEED: f32 = 8.0;
const MAX_SPEED: f32 = 1800.0;
const TICK: Duration = Duration::from_millis(16);

/// Scroll velocity while the pointer is held outside the viewport during a drag.
pub(super) struct DragScroll {
    last_tick: Option<Instant>,
}

impl DragScroll {
    pub fn new() -> Self {
        Self { last_tick: None }
    }

    /// The offset change for this pass (in pixels) and the pointer clamped into the
    /// viewport, or `None` when no drag is in progress outside it. Schedules the next
    /// tick, so a held pointer keeps scrolling without further input and an idle field
    /// requests nothing.
    pub fn step(
        &mut self,
        ctx: &mut Context,
        id: Id,
        view: Rect,
        axes: [bool; 2],
    ) -> Option<(Vec2, Vec2)> {
        let pointer = ctx.text_edit_pointer(id).filter(|_| ctx.active(id));
        let Some(pointer) = pointer.filter(|_| !view.is_empty()) else {
            self.last_tick = None;
            return None;
        };
        let mut outside = Vec2::ZERO;
        for axis in 0..2 {
            if axes[axis] {
                outside[axis] = (pointer[axis] - view.max[axis]).max(0.0)
                    + (pointer[axis] - view.min[axis]).min(0.0);
            }
        }
        if outside == Vec2::ZERO {
            self.last_tick = None;
            return None;
        }
        let now = ctx.frame_time();
        let seconds = self.last_tick.map_or(TICK, |last| {
            now.saturating_duration_since(last).min(Duration::from_millis(100))
        });
        self.last_tick = Some(now);
        ctx.request_repaint_after(TICK);
        let velocity = (outside * SPEED).clamp(Vec2::splat(-MAX_SPEED), Vec2::splat(MAX_SPEED));
        Some((velocity * seconds.as_secs_f32(), pointer.clamp(view.min, view.max)))
    }
}
