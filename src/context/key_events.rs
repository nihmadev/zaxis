//! Keys addressed to one widget: what a widget asks for and what it gets.
//!
//! A widget declares a [`KeyInterest`] for the keys it needs while it has focus. The
//! dispatcher reads the declarations of the last finished pass, so it can decide the owner
//! of a key the moment the event arrives, before any widget runs again. The owner finds the
//! [`KeyEvent`]s in the next pass.

use crate::actions::Mods;
use winit::{
    event::ElementState,
    keyboard::{Key, KeyCode, ModifiersState, PhysicalKey, SmolStr},
};

const ARROWS: [KeyCode; 4] = [
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
];
const NAVIGATION: [KeyCode; 8] = [
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::Home,
    KeyCode::End,
    KeyCode::PageUp,
    KeyCode::PageDown,
];
const ACTIVATION: [KeyCode; 3] = [KeyCode::Enter, KeyCode::NumpadEnter, KeyCode::Space];

/// One press, repeat or release of a key the focused widget claimed, in the order the
/// events arrived. The modifiers are those held when the event came, not when the pass
/// runs, so `Shift` released a moment later does not change what the press meant.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct KeyEvent {
    /// The key as navigation sees it: a numpad key with NumLock off is an arrow.
    pub code: KeyCode,
    /// The key as shortcuts see it: a letter is the key its Latin letter names, so
    /// `Ctrl+Z` follows the printed letter on any layout. Equal to `code` for other keys.
    pub layout: KeyCode,
    /// The scan-code based key, as the platform reported it.
    pub physical: PhysicalKey,
    /// What the key means with the current layout. [`Key::Unidentified`] when the host
    /// feeds physical codes only ([`Context::on_key_event`](crate::Context::on_key_event)).
    pub logical: Key,
    pub state: ElementState,
    /// The platform's autorepeat of a held key.
    pub repeat: bool,
    pub modifiers: ModifiersState,
    /// The text the press produces, if any. A claimed key does not type it.
    pub text: Option<SmolStr>,
}

impl KeyEvent {
    /// The initial press of a key, not its autorepeat.
    pub fn is_press(&self) -> bool {
        self.state == ElementState::Pressed && !self.repeat
    }

    /// An autorepeat of a held key. Delivered only to an interest that asked for
    /// [`KeyInterest::repeats`].
    pub fn is_repeat(&self) -> bool {
        self.state == ElementState::Pressed && self.repeat
    }

    pub fn is_release(&self) -> bool {
        self.state == ElementState::Released
    }

    /// The modifiers held, in the keymap's vocabulary. Never contains `Mods::PRIMARY`.
    pub fn mods(&self) -> Mods {
        Mods::from_state(self.modifiers)
    }
}

/// The keys a focused widget owns. Declare it every pass with
/// [`Ui::claim_keys`](crate::Ui::claim_keys) or [`Ui::keys`](crate::Ui::keys).
///
/// Declaring a key means owning it while the widget has focus: the dispatcher consumes
/// each matching press and its autorepeat and release, and Actions, containers and the
/// host never see them, even when the widget ignores an event. A key that no interest
/// names takes the usual path. By default an interest matches the key with no modifier
/// held and delivers presses only.
///
/// ```
/// # use zaxis::{KeyInterest, Mods};
/// # use zaxis::winit::keyboard::KeyCode;
/// // Arrows that repeat while held, and Ctrl+Z.
/// let step = KeyInterest::arrows().repeats();
/// let undo = KeyInterest::keys(&[KeyCode::KeyZ]).with_mods(Mods::PRIMARY);
/// # let _ = (step, undo);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct KeyInterest<'a> {
    pub(crate) keys: &'a [KeyCode],
    /// `None` matches any modifiers.
    pub(crate) mods: Option<Mods>,
    pub(crate) by_position: bool,
    pub(crate) repeats: bool,
    pub(crate) releases: bool,
}

impl<'a> KeyInterest<'a> {
    /// These keys, with no modifier held. Letters match the printed letter (see
    /// [`KeyEvent::layout`]), every other key its position.
    pub const fn keys(keys: &'a [KeyCode]) -> Self {
        Self {
            keys,
            mods: Some(Mods::NONE),
            by_position: false,
            repeats: false,
            releases: false,
        }
    }

    pub const fn arrows() -> KeyInterest<'static> {
        KeyInterest::keys(&ARROWS)
    }

    /// Arrows, Home, End, Page Up and Page Down.
    pub const fn navigation() -> KeyInterest<'static> {
        KeyInterest::keys(&NAVIGATION)
    }

    /// Enter, numpad Enter and Space. Claiming them replaces the click that Enter and
    /// Space raise on a focused region.
    pub const fn activation() -> KeyInterest<'static> {
        KeyInterest::keys(&ACTIVATION)
    }

    /// Match exactly these modifiers instead of none. [`Mods::PRIMARY`] is Command on
    /// macOS and Control elsewhere. AltGr, which some platforms report as Ctrl+Alt, never
    /// matches an interest for plain keys.
    pub const fn with_mods(mut self, mods: Mods) -> Self {
        self.mods = Some(mods);
        self
    }

    /// Match with any modifiers held. A claim this wide takes the key from every
    /// shortcut that uses it, so prefer exact modifiers.
    pub const fn any_mods(mut self) -> Self {
        self.mods = None;
        self
    }

    /// Match letters by the key's position (`KeyEvent::code`), as game controls do,
    /// instead of the printed letter.
    pub const fn by_position(mut self) -> Self {
        self.by_position = true;
        self
    }

    /// Also deliver the autorepeat of a held key. Without it repeats are still owned and
    /// consumed, so they neither step a control twice nor reach anything else.
    pub const fn repeats(mut self) -> Self {
        self.repeats = true;
        self
    }

    /// Also deliver the release of a claimed key, in order with the presses. The release
    /// is owned and consumed either way, even after focus moved elsewhere.
    pub const fn releases(mut self) -> Self {
        self.releases = true;
        self
    }
}
