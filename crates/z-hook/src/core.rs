//! State shared by every thread that touches the overlay: options, visibility, the queue of
//! input, counters, and the interface closure until a render thread takes it.

use crate::{
    driver,
    input::{decide, Delivery, InputHub},
    options::{OverlayInput, OverlayOptions, ToggleKey},
    stats::{Counters, OverlayStats},
};
use std::{
    sync::{
        atomic::{
            AtomicBool,
            Ordering::{Acquire, Relaxed, Release},
        },
        Arc, Mutex, MutexGuard, PoisonError,
    },
    time::Instant,
};
use zaxis::{
    winit::{event::ElementState, keyboard::PhysicalKey},
    Context, InputEvent,
};

pub(crate) type UiFn = Box<dyn FnMut(&mut Context) + Send + 'static>;
pub(crate) type Clock = Arc<dyn Fn() -> Instant + Send + Sync>;

pub(crate) struct Core {
    pub options: OverlayOptions,
    pub hub: InputHub,
    pub counters: Counters,
    pub clock: Clock,
    visible: AtomicBool,
    disabled: AtomicBool,
    reason: Mutex<Option<String>>,
    ui: Mutex<Option<UiFn>>,
}

pub(crate) fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Core {
    pub fn new(options: OverlayOptions, ui: UiFn, clock: Clock) -> Arc<Self> {
        Arc::new(Self {
            visible: AtomicBool::new(options.start_visible),
            options,
            hub: InputHub::default(),
            counters: Counters::default(),
            clock,
            disabled: AtomicBool::new(false),
            reason: Mutex::new(None),
            ui: Mutex::new(Some(ui)),
        })
    }

    pub fn now(&self) -> Instant {
        (self.clock)()
    }

    pub fn is_visible(&self) -> bool {
        self.visible.load(Acquire)
    }

    pub fn set_visible(&self, visible: bool) {
        self.visible.store(visible, Release);
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.load(Acquire)
    }

    pub fn disabled_reason(&self) -> Option<String> {
        lock(&self.reason).clone()
    }

    /// Switch the overlay off for good and say why; the host goes on unchanged.
    pub fn disable(&self, reason: impl Into<String>) {
        let reason = reason.into();
        if !self.disabled.swap(true, Release) {
            log::error!("z-hook: overlay disabled: {reason}");
            *lock(&self.reason) = Some(reason);
            self.hub.forget_held();
        }
    }

    pub fn stats(&self) -> OverlayStats {
        self.counters.snapshot()
    }

    /// The interface closure goes to the first thread that presents, and back when that
    /// thread ends, so another can adopt it.
    pub fn take_ui(&self) -> Option<UiFn> {
        lock(&self.ui).take()
    }

    pub fn return_ui(&self, ui: UiFn) {
        *lock(&self.ui) = Some(ui);
    }

    fn toggles(&self, event: &InputEvent) -> bool {
        let (Some(ToggleKey { key, modifiers }), InputEvent::Key(input)) =
            (self.options.toggle, event)
        else {
            return false;
        };
        input.physical == PhysicalKey::Code(key)
            && input.state == ElementState::Pressed
            && !input.repeat
            && self.hub.modifiers() == modifiers
    }

    /// Decide who gets `event`, and give it to the interface when it is visible. Callable
    /// from any thread (a window procedure, an X11 reader). On the render thread, outside a
    /// frame, the interface answers at once; elsewhere the event waits in the queue for the
    /// next frame and the answer is estimated from the last one.
    pub fn route(self: &Arc<Self>, event: InputEvent) -> Delivery {
        if self.is_disabled() {
            return Delivery::HOST;
        }
        self.hub.observe(&event);
        if self.toggles(&event) {
            self.set_visible(!self.is_visible());
            let pass = self.options.input == OverlayInput::PassThrough;
            if !pass {
                self.hub.press_taken(&event);
            }
            return Delivery {
                overlay: false,
                host: pass,
            };
        }
        if self.hub.release_is_ours(&event) {
            self.deliver(event);
            return Delivery::OVERLAY;
        }
        if !self.is_visible() {
            return Delivery::HOST;
        }
        let release = matches!(&event, InputEvent::Key(k) if k.state == ElementState::Released)
            || matches!(
                &event,
                InputEvent::Button {
                    state: ElementState::Released,
                    ..
                }
            );
        if release {
            // A release of a press the host saw: it has to see the release too.
            self.deliver(event);
            return Delivery::BOTH;
        }
        let always_shared = matches!(
            event,
            InputEvent::Modifiers(_)
                | InputEvent::Focus(_)
                | InputEvent::Resized { .. }
                | InputEvent::ScaleFactor(_)
        );
        let estimate = self.estimate(&event);
        let consumed = self.deliver(event.clone()).unwrap_or(estimate);
        if always_shared {
            return Delivery::BOTH;
        }
        let delivery = decide(self.options.input, true, consumed);
        if !delivery.host {
            self.hub.press_taken(&event);
        }
        delivery
    }

    /// Hand an event to the interface now (`Some(consumed)`) or queue it (`None`).
    fn deliver(self: &Arc<Self>, event: InputEvent) -> Option<bool> {
        match driver::dispatch_now(self, &event) {
            Some(consumed) => Some(consumed),
            None => {
                if self.hub.push(event) {
                    Counters::bump(&self.counters.dropped_input);
                }
                None
            }
        }
    }

    fn estimate(&self, event: &InputEvent) -> bool {
        match event {
            InputEvent::PointerMoved { .. } | InputEvent::Button { .. } | InputEvent::Wheel(_) => {
                self.hub.policy.pointer_over_ui.load(Relaxed)
            }
            InputEvent::Key(_) | InputEvent::Ime(_) => self.hub.policy.keyboard_focus.load(Relaxed),
            _ => false,
        }
    }
}
