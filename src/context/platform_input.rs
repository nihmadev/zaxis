//! Window-system independent input: the events a [`Context`](super::Context) understands,
//! without a window or an event loop.
//!
//! The winit runner converts every [`WindowEvent`] into an [`InputEvent`] and hands it to
//! [`Context::on_input`](super::Context::on_input); a host that has no winit window (an
//! overlay inside another process, a test, a game engine with its own event source) builds
//! `InputEvent`s itself and takes the same path, so both behave identically. The key and
//! button vocabulary is winit's (`KeyCode`, `Key`, `MouseButton`, `ElementState`,
//! `ModifiersState`): plain data enums that need no window.

use crate::Vec2;
use std::path::PathBuf;
use winit::{
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, WindowEvent},
    keyboard::{Key, ModifiersState, PhysicalKey, SmolStr},
};

/// One key press, release or repeat.
///
/// `physical` is the scan-code based key (shortcuts, game-style bindings); `logical` is what
/// the key means with the current layout (navigation keys, characters); `text` is the text the
/// press produces, if any, for text fields. Navigation follows `logical` (a numpad key with
/// NumLock off is an arrow), letter shortcuts follow `physical`.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyInput {
    pub physical: PhysicalKey,
    pub logical: Key,
    pub state: ElementState,
    pub repeat: bool,
    pub text: Option<SmolStr>,
}

/// A wheel or touchpad scroll.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WheelDelta {
    /// Notches of a mouse wheel: `x` right, `y` up, in lines.
    Lines(Vec2),
    /// Smooth scrolling in physical pixels.
    Pixels(Vec2),
}

/// Input method composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImeEvent {
    /// The text being composed and the selected byte range in it, if the input method
    /// reports one. Empty text clears the composition.
    Preedit(String, Option<(usize, usize)>),
    /// The composition ended with this text.
    Commit(String),
    /// The input method was turned off: the composition ends.
    Disabled,
}

/// An input event addressed to one [`Context`](super::Context). Positions and sizes are
/// physical pixels of the viewport the context draws; the context divides by its scale factor.
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// The pointer is at this position (physical pixels, origin top-left of the viewport).
    PointerMoved {
        x: f64,
        y: f64,
    },
    /// The pointer left the viewport.
    PointerLeft,
    Button {
        button: MouseButton,
        state: ElementState,
    },
    Wheel(WheelDelta),
    Key(KeyInput),
    /// The modifier keys held now. Send it whenever they change, before the key that needs them.
    Modifiers(ModifiersState),
    Ime(ImeEvent),
    /// The viewport gained or lost keyboard focus. Losing it releases every held key and
    /// button, closes popups and cancels drags.
    Focus(bool),
    /// The viewport has a new size in physical pixels; the scale factor is kept.
    Resized {
        width: u32,
        height: u32,
    },
    /// The scale factor (DPI) changed; the physical size is kept.
    ScaleFactor(f64),
    FileHovered(PathBuf),
    FileDropped(PathBuf),
    FileHoverCancelled,
}

impl InputEvent {
    /// The event a winit window event stands for, or `None` for events the context does not
    /// use (redraws, theme, touch and the like).
    pub fn from_window_event(event: &WindowEvent) -> Option<Self> {
        Some(match event {
            WindowEvent::CursorMoved { position, .. } => Self::PointerMoved {
                x: position.x,
                y: position.y,
            },
            WindowEvent::CursorLeft { .. } => Self::PointerLeft,
            WindowEvent::MouseInput { state, button, .. } => Self::Button {
                button: *button,
                state: *state,
            },
            WindowEvent::MouseWheel { delta, .. } => Self::Wheel(match delta {
                MouseScrollDelta::LineDelta(x, y) => WheelDelta::Lines(Vec2::new(*x, *y)),
                MouseScrollDelta::PixelDelta(p) => {
                    WheelDelta::Pixels(Vec2::new(p.x as f32, p.y as f32))
                }
            }),
            WindowEvent::KeyboardInput { event, .. } => Self::Key(KeyInput {
                physical: event.physical_key,
                logical: event.logical_key.clone(),
                state: event.state,
                repeat: event.repeat,
                text: event.text.clone(),
            }),
            WindowEvent::ModifiersChanged(modifiers) => Self::Modifiers(modifiers.state()),
            WindowEvent::Ime(Ime::Commit(text)) => Self::Ime(ImeEvent::Commit(text.clone())),
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                Self::Ime(ImeEvent::Preedit(text.clone(), *cursor))
            }
            WindowEvent::Ime(Ime::Disabled) => Self::Ime(ImeEvent::Disabled),
            WindowEvent::Focused(focused) => Self::Focus(*focused),
            WindowEvent::Resized(size) => Self::Resized {
                width: size.width,
                height: size.height,
            },
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                Self::ScaleFactor(*scale_factor)
            }
            WindowEvent::HoveredFile(path) => Self::FileHovered(path.clone()),
            WindowEvent::DroppedFile(path) => Self::FileDropped(path.clone()),
            WindowEvent::HoveredFileCancelled => Self::FileHoverCancelled,
            _ => return None,
        })
    }
}
