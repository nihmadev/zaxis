use std::time::Duration;
use zaxis::{Easing, Tween, Ui};

const DURATION: Duration = Duration::from_millis(700);

/// Scrolling an area to a position over time, with the animation engine: the tween's value is
/// handed to `ScrollArea::scroll_offset` on every pass until it completes. Any wheel or
/// pointer input from the user ends it at once, so the page never fights back.
#[derive(Default)]
pub struct SmoothScroll {
    serial: u64,
    motion: Option<(f32, f32)>,
}

impl SmoothScroll {
    /// Scroll from the current `offset` to `target` (both in content pixels).
    pub fn start(&mut self, offset: f32, target: f32) {
        self.serial += 1;
        self.motion = Some((offset, target));
    }

    /// The offset to apply on this pass, or `None` when the user is in control.
    pub fn offset(&mut self, ui: &mut Ui<'_>) -> Option<f32> {
        let (from, to) = self.motion?;
        let interrupted = {
            let input = ui.context().input();
            input.scroll_delta.length_squared() > 0.0 || input.primary_down
        };
        if interrupted {
            self.motion = None;
            return None;
        }
        let step = ui.animate(("smooth-scroll", self.serial), || {
            Tween::new(from, to, DURATION).easing(Easing::CubicInOut)
        });
        if !step.running() {
            self.motion = None;
        }
        Some(step.value)
    }
}
