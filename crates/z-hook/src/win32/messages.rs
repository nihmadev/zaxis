//! The translator from window messages to input events.

use super::{
    keys::{logical_key, modifier_of, x_button},
    scan::scan_code_to_key,
    wm,
};
use zaxis::{
    vec2,
    winit::{
        event::{ElementState, MouseButton},
        keyboard::{Key, ModifiersState, NativeKeyCode, PhysicalKey},
    },
    ImeEvent, InputEvent, KeyInput, WheelDelta,
};

/// What a message is about, which decides whether the host may be denied it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Mouse buttons, movement and wheel.
    Pointer,
    /// Key presses and releases.
    Keyboard,
    /// `WM_CHAR` and its relatives.
    Text,
    /// Input method composition.
    Ime,
    /// `WM_INPUT`, raw device data: games read mouse deltas from it.
    Raw,
    /// `WM_SETCURSOR`: who picks the cursor.
    Cursor,
    /// Focus, DPI and leave notifications, which every window needs.
    Notification,
    Other,
}

/// What a message came to: its class and the events it stands for (usually one).
#[derive(Clone, Debug, PartialEq)]
pub struct Translation {
    pub class: Class,
    pub events: Vec<InputEvent>,
}

impl Translation {
    fn of(class: Class, events: Vec<InputEvent>) -> Self {
        Self { class, events }
    }

    fn one(class: Class, event: InputEvent) -> Self {
        Self::of(class, vec![event])
    }
}

/// Turns the messages of one window into [`InputEvent`]s. It keeps what messages alone do not
/// say: the modifiers held and the first half of a surrogate pair.
#[derive(Debug, Default)]
pub struct Translator {
    modifiers: ModifiersState,
    high_surrogate: Option<u16>,
}

impl Translator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seed the modifiers from the system (`GetKeyState`) when the hook attaches.
    pub fn set_modifiers(&mut self, modifiers: ModifiersState) {
        self.modifiers = modifiers;
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    /// Translate one message. Messages that mean nothing for input come back as
    /// [`Class::Other`] with no events.
    pub fn translate(&mut self, msg: u32, wparam: usize, lparam: isize) -> Translation {
        match msg {
            wm::MOUSEMOVE => {
                let (x, y) = client_position(lparam);
                Translation::one(Class::Pointer, InputEvent::PointerMoved { x, y })
            }
            wm::MOUSELEAVE => Translation::one(Class::Notification, InputEvent::PointerLeft),
            wm::LBUTTONDOWN | wm::LBUTTONDBLCLK => button(MouseButton::Left, true, lparam),
            wm::LBUTTONUP => button(MouseButton::Left, false, lparam),
            wm::RBUTTONDOWN | wm::RBUTTONDBLCLK => button(MouseButton::Right, true, lparam),
            wm::RBUTTONUP => button(MouseButton::Right, false, lparam),
            wm::MBUTTONDOWN | wm::MBUTTONDBLCLK => button(MouseButton::Middle, true, lparam),
            wm::MBUTTONUP => button(MouseButton::Middle, false, lparam),
            wm::XBUTTONDOWN | wm::XBUTTONDBLCLK => button(x_button(wparam), true, lparam),
            wm::XBUTTONUP => button(x_button(wparam), false, lparam),
            wm::MOUSEWHEEL | wm::MOUSEHWHEEL => {
                let notches = f32::from(((wparam >> 16) & 0xFFFF) as u16 as i16) / 120.0;
                let delta = if msg == wm::MOUSEWHEEL {
                    vec2(0.0, notches)
                } else {
                    vec2(notches, 0.0)
                };
                Translation::one(Class::Pointer, InputEvent::Wheel(WheelDelta::Lines(delta)))
            }
            wm::KEYDOWN | wm::SYSKEYDOWN => self.key(wparam, lparam, ElementState::Pressed),
            wm::KEYUP | wm::SYSKEYUP => self.key(wparam, lparam, ElementState::Released),
            wm::CHAR | wm::SYSCHAR => self.text(wparam as u32, true),
            wm::UNICHAR => self.text(wparam as u32, false),
            wm::SETFOCUS => Translation::one(Class::Notification, InputEvent::Focus(true)),
            wm::KILLFOCUS => {
                self.modifiers = ModifiersState::empty();
                self.high_surrogate = None;
                Translation::of(
                    Class::Notification,
                    vec![
                        InputEvent::Modifiers(ModifiersState::empty()),
                        InputEvent::Focus(false),
                    ],
                )
            }
            wm::DPICHANGED => {
                let dpi = f64::from((wparam & 0xFFFF) as u16);
                Translation::one(Class::Notification, InputEvent::ScaleFactor(dpi / 96.0))
            }
            wm::INPUT => Translation::of(Class::Raw, Vec::new()),
            wm::SETCURSOR => Translation::of(Class::Cursor, Vec::new()),
            wm::IME_STARTCOMPOSITION | wm::IME_COMPOSITION | wm::IME_CHAR => {
                Translation::of(Class::Ime, Vec::new())
            }
            wm::IME_ENDCOMPOSITION => Translation::one(
                Class::Ime,
                InputEvent::Ime(ImeEvent::Preedit(String::new(), None)),
            ),
            _ => Translation::of(Class::Other, Vec::new()),
        }
    }

    fn key(&mut self, wparam: usize, lparam: isize, state: ElementState) -> Translation {
        let scan = ((lparam >> 16) & 0xFF) as u16;
        let extended = (lparam >> 24) & 1 == 1;
        let repeat = state == ElementState::Pressed && (lparam >> 30) & 1 == 1;
        let vk = (wparam & 0xFFFF) as u16;
        let physical = match scan_code_to_key(scan, extended) {
            Some(code) => PhysicalKey::Code(code),
            None => PhysicalKey::Unidentified(NativeKeyCode::Windows(scan)),
        };
        let mut events = Vec::with_capacity(2);
        if let PhysicalKey::Code(code) = physical {
            if let Some(modifier) = modifier_of(code) {
                let mut modifiers = self.modifiers;
                modifiers.set(modifier, state == ElementState::Pressed);
                if modifiers != self.modifiers {
                    self.modifiers = modifiers;
                    events.push(InputEvent::Modifiers(modifiers));
                }
            }
        }
        events.push(InputEvent::Key(KeyInput {
            physical,
            logical: logical_key(vk),
            state,
            repeat,
            text: None,
        }));
        Translation::of(Class::Keyboard, events)
    }

    /// `unit` is a UTF-16 code unit for `WM_CHAR`, a code point for `WM_UNICHAR`. Control
    /// characters are keys, not text; lone surrogates are dropped.
    fn text(&mut self, unit: u32, utf16: bool) -> Translation {
        let character = match unit {
            0xD800..=0xDBFF if utf16 => {
                self.high_surrogate = Some(unit as u16);
                None
            }
            0xDC00..=0xDFFF if utf16 => self.high_surrogate.take().and_then(|high| {
                char::decode_utf16([high, unit as u16])
                    .next()
                    .and_then(Result::ok)
            }),
            _ => {
                self.high_surrogate = None;
                char::from_u32(unit)
            }
        };
        match character.filter(|c| !c.is_control()) {
            Some(character) => {
                let text = character.to_string();
                Translation::one(
                    Class::Text,
                    InputEvent::Key(KeyInput {
                        physical: PhysicalKey::Unidentified(NativeKeyCode::Unidentified),
                        logical: Key::Character(text.as_str().into()),
                        state: ElementState::Pressed,
                        repeat: false,
                        text: Some(text.as_str().into()),
                    }),
                )
            }
            None => Translation::of(Class::Text, Vec::new()),
        }
    }
}

/// The events for the strings of a `WM_IME_COMPOSITION`: the finished text first, then the
/// text still being composed with the byte range of its caret.
pub fn ime_composition(
    result: Option<&str>,
    composing: Option<(&str, Option<(usize, usize)>)>,
) -> Vec<InputEvent> {
    let mut events = Vec::new();
    if let Some(text) = result.filter(|text| !text.is_empty()) {
        events.push(InputEvent::Ime(ImeEvent::Commit(text.to_owned())));
    }
    if let Some((text, cursor)) = composing {
        events.push(InputEvent::Ime(ImeEvent::Preedit(text.to_owned(), cursor)));
    }
    events
}

fn client_position(lparam: isize) -> (f64, f64) {
    let x = (lparam & 0xFFFF) as u16 as i16;
    let y = ((lparam >> 16) & 0xFFFF) as u16 as i16;
    (f64::from(x), f64::from(y))
}

fn button(button: MouseButton, down: bool, lparam: isize) -> Translation {
    let (x, y) = client_position(lparam);
    let state = if down {
        ElementState::Pressed
    } else {
        ElementState::Released
    };
    Translation::of(
        Class::Pointer,
        vec![
            InputEvent::PointerMoved { x, y },
            InputEvent::Button { button, state },
        ],
    )
}
