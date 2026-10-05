use std::{hash::Hash, time::Duration};
use zaxis::{vec2, Easing, Rect, Transform, TweenOptions, Ui};

const RISE: f32 = 24.0;

/// Fade a block in while it moves up into place, `delay` after it first appears. The block
/// keeps its layout space from the first pass, so nothing shifts when it arrives.
pub fn fade_up<R>(
    ui: &mut Ui<'_>,
    source: impl Hash + Copy,
    delay: Duration,
    build: impl FnOnce(&mut Ui<'_>) -> R,
) -> R {
    let options = TweenOptions::new(Duration::from_millis(700))
        .easing(Easing::CubicOut)
        .delay(delay);
    let t = ui.transition_from(source, 0.0_f32, 1.0, options).value;
    ui.visual(
        source,
        Transform::translation(vec2(0.0, (1.0 - t) * RISE)),
        t,
        build,
    )
}

/// Which blocks of a page have scrolled into view, so each can play its entrance once.
#[derive(Default)]
pub struct Reveal {
    seen: Vec<bool>,
    frames: Vec<Option<Rect>>,
}

impl Reveal {
    /// Run `build` as block `index`. The block enters the first time its frame from the
    /// previous pass overlaps the visible part of the scroll area; `delay` staggers blocks
    /// that enter together. `build` returns the block's frame.
    pub fn block(
        &mut self,
        ui: &mut Ui<'_>,
        index: usize,
        delay: Duration,
        build: impl FnOnce(&mut Ui<'_>) -> Rect,
    ) {
        if self.seen.len() <= index {
            self.seen.resize(index + 1, false);
            self.frames.resize(index + 1, None);
        }
        let visible = ui.clip_rect();
        let on_screen =
            self.frames[index].is_some_and(|frame| !frame.intersect(visible).is_empty());
        self.seen[index] |= on_screen;
        let options = TweenOptions::new(Duration::from_millis(600))
            .easing(Easing::CubicOut)
            .delay(delay);
        let target = if self.seen[index] { 1.0_f32 } else { 0.0 };
        let t = ui.transition(("reveal", index), target, options).value;
        let frame = ui.visual(
            ("reveal", index),
            Transform::translation(vec2(0.0, (1.0 - t) * RISE)),
            t,
            build,
        );
        if self.frames[index].replace(frame) != Some(frame) {
            // The frame was unknown or moved: look again with the new geometry.
            ui.context().request_repaint();
        }
    }
}
