//! What the host's window procedure does with a message, given what the overlay decided about
//! the events in it.

use super::Class;
use crate::{input::Delivery, options::OverlayInput};

/// The overlay's state the decision depends on.
#[derive(Clone, Copy, Debug)]
pub struct Situation {
    pub mode: OverlayInput,
    pub visible: bool,
    pub pointer_over_ui: bool,
    pub keyboard_focus: bool,
}

impl Situation {
    /// The overlay claims the pointer: in `CaptureWhenVisible` while shown, in
    /// `CaptureWhenFocused` while it is over the interface.
    pub fn takes_pointer(&self) -> bool {
        self.visible
            && match self.mode {
                OverlayInput::PassThrough => false,
                OverlayInput::CaptureWhenVisible => true,
                OverlayInput::CaptureWhenFocused => self.pointer_over_ui,
            }
    }

    /// The overlay claims the keyboard and the input method likewise.
    pub fn takes_keyboard(&self) -> bool {
        self.visible
            && match self.mode {
                OverlayInput::PassThrough => false,
                OverlayInput::CaptureWhenVisible => true,
                OverlayInput::CaptureWhenFocused => self.keyboard_focus,
            }
    }
}

/// Whether the host's window procedure gets the message.
///
/// Messages with events go where their events went (all of them must allow it); raw device
/// input follows the pointer, and input method messages follow the keyboard.
pub fn host_gets(class: Class, deliveries: &[Delivery], situation: Situation) -> bool {
    match class {
        Class::Raw => !situation.takes_pointer(),
        Class::Ime => !situation.takes_keyboard(),
        Class::Cursor | Class::Notification | Class::Other => true,
        Class::Pointer | Class::Keyboard | Class::Text => deliveries.iter().all(|d| d.host),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn situation(mode: OverlayInput, visible: bool, pointer: bool, keyboard: bool) -> Situation {
        Situation {
            mode,
            visible,
            pointer_over_ui: pointer,
            keyboard_focus: keyboard,
        }
    }

    #[test]
    fn raw_input_follows_the_pointer_and_ime_the_keyboard() {
        use OverlayInput::*;
        assert!(
            host_gets(
                Class::Raw,
                &[],
                situation(CaptureWhenFocused, true, false, true)
            ),
            "mouse look goes on while the pointer is not over the interface"
        );
        assert!(!host_gets(
            Class::Raw,
            &[],
            situation(CaptureWhenFocused, true, true, false)
        ));
        assert!(!host_gets(
            Class::Raw,
            &[],
            situation(CaptureWhenVisible, true, false, false)
        ));
        assert!(
            host_gets(
                Class::Raw,
                &[],
                situation(CaptureWhenVisible, false, true, true)
            ),
            "hidden: the host has it all"
        );
        assert!(host_gets(
            Class::Raw,
            &[],
            situation(PassThrough, true, true, true)
        ));
        assert!(!host_gets(
            Class::Ime,
            &[],
            situation(CaptureWhenFocused, true, false, true)
        ));
        assert!(host_gets(
            Class::Ime,
            &[],
            situation(CaptureWhenFocused, true, true, false)
        ));
    }

    #[test]
    fn a_message_reaches_the_host_only_if_all_its_events_may() {
        let s = situation(OverlayInput::CaptureWhenFocused, true, true, false);
        assert!(host_gets(
            Class::Pointer,
            &[Delivery::BOTH, Delivery::BOTH],
            s
        ));
        assert!(!host_gets(
            Class::Pointer,
            &[Delivery::BOTH, Delivery::OVERLAY],
            s
        ));
        assert!(
            host_gets(Class::Notification, &[Delivery::OVERLAY], s),
            "focus and DPI notices always reach the host"
        );
    }

    #[test]
    fn claims_follow_the_mode() {
        assert!(situation(OverlayInput::CaptureWhenVisible, true, false, false).takes_pointer());
        assert!(!situation(OverlayInput::CaptureWhenFocused, true, false, true).takes_pointer());
        assert!(situation(OverlayInput::CaptureWhenFocused, true, false, true).takes_keyboard());
        assert!(!situation(OverlayInput::PassThrough, true, true, true).takes_pointer());
    }
}
