//! Input on its way to the overlay: the queue, what is known about the interface between
//! frames, and who gets each event.

use crate::options::OverlayInput;
use std::{
    collections::{HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering::Relaxed},
        Mutex, MutexGuard, PoisonError,
    },
};
use zaxis::{
    winit::{
        event::{ElementState, MouseButton},
        keyboard::{ModifiersState, PhysicalKey},
    },
    InputEvent,
};

/// Beyond this many queued events, moves and wheel steps are dropped; presses and keys always
/// queue (up to a hard limit), so a stalled render thread cannot lose a click.
const SOFT_LIMIT: usize = 1024;
const HARD_LIMIT: usize = 16 * 1024;

/// Which of the two consumers gets an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Delivery {
    pub overlay: bool,
    pub host: bool,
}

impl Delivery {
    pub const HOST: Self = Self {
        overlay: false,
        host: true,
    };
    pub const OVERLAY: Self = Self {
        overlay: true,
        host: false,
    };
    pub const BOTH: Self = Self {
        overlay: true,
        host: true,
    };
}

/// Who gets an event the overlay did (`consumed`) or did not take, while it is `visible`.
pub fn decide(mode: OverlayInput, visible: bool, consumed: bool) -> Delivery {
    match (visible, mode) {
        (false, _) => Delivery::HOST,
        (true, OverlayInput::PassThrough) => Delivery::BOTH,
        (true, OverlayInput::CaptureWhenVisible) => Delivery::OVERLAY,
        (true, OverlayInput::CaptureWhenFocused) => Delivery {
            overlay: true,
            host: !consumed,
        },
    }
}

/// What the overlay's last frame left known, for threads that cannot ask the interface.
#[derive(Default)]
pub(crate) struct Policy {
    pub pointer_over_ui: AtomicBool,
    pub keyboard_focus: AtomicBool,
    /// The stock cursor (`IDC_*`) the interface wants under the pointer.
    pub cursor: AtomicU32,
    /// Where text is being entered, in physical pixels (`[x, y, width, height]`).
    pub ime_area: Mutex<Option<[f32; 4]>>,
}

/// Presses the overlay took, whose releases it must take too (or the host would see a release
/// without a press), and the pointer and modifiers as last seen.
#[derive(Default)]
struct Held {
    keys: HashSet<PhysicalKey>,
    buttons: HashSet<MouseButton>,
    pointer: Option<(f64, f64)>,
}

#[derive(Default)]
pub(crate) struct InputHub {
    queue: Mutex<VecDeque<InputEvent>>,
    held: Mutex<Held>,
    pub policy: Policy,
    modifiers: AtomicU32,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl InputHub {
    /// Queue an event for the next frame. Returns whether it was dropped.
    pub fn push(&self, event: InputEvent) -> bool {
        let mut queue = lock(&self.queue);
        match (&event, queue.back_mut()) {
            (InputEvent::PointerMoved { .. }, Some(last @ InputEvent::PointerMoved { .. })) => {
                *last = event;
                return false;
            }
            (InputEvent::Wheel(new), Some(InputEvent::Wheel(old))) if same_unit(old, new) => {
                *old = add_wheel(*old, *new);
                return false;
            }
            _ => {}
        }
        let droppable = matches!(
            event,
            InputEvent::PointerMoved { .. } | InputEvent::Wheel(_)
        );
        if queue.len() >= if droppable { SOFT_LIMIT } else { HARD_LIMIT } {
            return true;
        }
        queue.push_back(event);
        false
    }

    pub fn drain(&self) -> VecDeque<InputEvent> {
        std::mem::take(&mut *lock(&self.queue))
    }

    #[cfg(test)]
    fn is_empty(&self) -> bool {
        lock(&self.queue).is_empty()
    }

    pub fn modifiers(&self) -> ModifiersState {
        ModifiersState::from_bits_truncate(self.modifiers.load(Relaxed))
    }

    /// Note state every event carries, whoever ends up taking it.
    pub fn observe(&self, event: &InputEvent) {
        match event {
            InputEvent::Modifiers(modifiers) => self.modifiers.store(modifiers.bits(), Relaxed),
            InputEvent::PointerMoved { x, y } => lock(&self.held).pointer = Some((*x, *y)),
            InputEvent::PointerLeft => lock(&self.held).pointer = None,
            _ => {}
        }
    }

    pub fn pointer(&self) -> Option<(f64, f64)> {
        lock(&self.held).pointer
    }

    /// Whether a release belongs to a press the overlay took (and forget that press).
    pub fn release_is_ours(&self, event: &InputEvent) -> bool {
        let mut held = lock(&self.held);
        match event {
            InputEvent::Key(key) if key.state == ElementState::Released => {
                held.keys.remove(&key.physical)
            }
            InputEvent::Button { button, state } if *state == ElementState::Released => {
                held.buttons.remove(button)
            }
            _ => false,
        }
    }

    /// Remember that the overlay took this press.
    pub fn press_taken(&self, event: &InputEvent) {
        let mut held = lock(&self.held);
        match event {
            InputEvent::Key(key) if key.state == ElementState::Pressed => {
                held.keys.insert(key.physical);
            }
            InputEvent::Button { button, state } if *state == ElementState::Pressed => {
                held.buttons.insert(*button);
            }
            _ => {}
        }
    }

    /// The overlay lost its right to the input (hidden, disabled): nothing it took stays held.
    pub fn forget_held(&self) {
        let mut held = lock(&self.held);
        held.keys.clear();
        held.buttons.clear();
    }
}

fn same_unit(a: &zaxis::WheelDelta, b: &zaxis::WheelDelta) -> bool {
    matches!(
        (a, b),
        (zaxis::WheelDelta::Lines(_), zaxis::WheelDelta::Lines(_))
            | (zaxis::WheelDelta::Pixels(_), zaxis::WheelDelta::Pixels(_))
    )
}

fn add_wheel(a: zaxis::WheelDelta, b: zaxis::WheelDelta) -> zaxis::WheelDelta {
    match (a, b) {
        (zaxis::WheelDelta::Lines(a), zaxis::WheelDelta::Lines(b)) => {
            zaxis::WheelDelta::Lines(a + b)
        }
        (zaxis::WheelDelta::Pixels(a), zaxis::WheelDelta::Pixels(b)) => {
            zaxis::WheelDelta::Pixels(a + b)
        }
        (_, b) => b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_decide_who_gets_an_event() {
        use OverlayInput::*;
        assert_eq!(decide(PassThrough, false, true), Delivery::HOST);
        assert_eq!(decide(PassThrough, true, true), Delivery::BOTH);
        assert_eq!(decide(CaptureWhenVisible, true, false), Delivery::OVERLAY);
        assert_eq!(decide(CaptureWhenVisible, false, true), Delivery::HOST);
        assert_eq!(
            decide(CaptureWhenFocused, true, true),
            Delivery {
                overlay: true,
                host: false
            }
        );
        assert_eq!(decide(CaptureWhenFocused, true, false), Delivery::BOTH);
    }

    #[test]
    fn queue_coalesces_moves_and_wheel_steps() {
        let hub = InputHub::default();
        hub.push(InputEvent::PointerMoved { x: 1.0, y: 1.0 });
        hub.push(InputEvent::PointerMoved { x: 5.0, y: 6.0 });
        hub.push(InputEvent::Wheel(zaxis::WheelDelta::Lines(zaxis::vec2(
            0.0, 1.0,
        ))));
        hub.push(InputEvent::Wheel(zaxis::WheelDelta::Lines(zaxis::vec2(
            0.0, 2.0,
        ))));
        let events: Vec<_> = hub.drain().into();
        assert_eq!(
            events,
            [
                InputEvent::PointerMoved { x: 5.0, y: 6.0 },
                InputEvent::Wheel(zaxis::WheelDelta::Lines(zaxis::vec2(0.0, 3.0))),
            ]
        );
        assert!(hub.is_empty());
    }

    #[test]
    fn moves_drop_at_the_soft_limit_but_presses_do_not() {
        let hub = InputHub::default();
        for i in 0..SOFT_LIMIT {
            hub.push(InputEvent::Focus(i % 2 == 0));
        }
        assert!(hub.push(InputEvent::PointerMoved { x: 0.0, y: 0.0 }));
        assert!(!hub.push(InputEvent::Button {
            button: MouseButton::Left,
            state: ElementState::Pressed
        }));
    }
}
