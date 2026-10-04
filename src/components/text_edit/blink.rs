//! Caret blinking driven by the animation engine's repaint deadlines, so an idle focused
//! field costs one wake-up per half period and an unfocused one costs nothing.
use super::*;
use crate::{AnimationOptions, AnimationSample, Procedural};

/// Whether the caret is in the visible half of its blink cycle. `activity` (input or a
/// focus change) restarts the cycle so the caret is solid while typing.
pub(super) fn caret_visible(
    ui: &mut Ui<'_>,
    id: Id,
    state: &TextEditState,
    interval: Duration,
    activity: bool,
    suspended: bool,
) -> bool {
    if interval.is_zero() || suspended {
        return true;
    }
    let blink_id = id.with("cursor-blink");
    let create = || {
        Procedural::new(true, move |elapsed: Duration| {
            let show = (elapsed.as_nanos() / interval.as_nanos()).is_multiple_of(2);
            let remainder = Duration::new(
                ((elapsed.as_nanos() % interval.as_nanos()) / 1_000_000_000) as u64,
                ((elapsed.as_nanos() % interval.as_nanos()) % 1_000_000_000) as u32,
            );
            AnimationSample::after(show, interval - remainder)
        })
    };
    let options = AnimationOptions { decorative: false };
    if activity || state.blink_interval != interval {
        ui.context
            .restart_animation_with(blink_id, create(), options);
    }
    ui.context.animate_with(blink_id, options, create).value
}
