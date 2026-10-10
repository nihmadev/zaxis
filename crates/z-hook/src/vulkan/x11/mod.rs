//! Input from the host's X11 window, by observing it.
//!
//! A second X connection selects XInput2 events on the host's window: the server delivers
//! them to every client that selected them, so the host sees all of them as before. The
//! overlay can react but cannot take anything away from the host (it would need a grab,
//! which would break the host); in [`OverlayInput::CaptureWhenVisible`](crate::OverlayInput)
//! the host still gets the input. Text is decoded from the server's keyboard mapping (see
//! [`keys::keysym_char`] for what it covers).

mod keys;

use crate::core::Core;
use keys::KeyMap;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering::Relaxed},
        Arc,
    },
    thread::JoinHandle,
};
use x11rb::{
    connection::Connection,
    protocol::{
        xinput::{self, ConnectionExt as _, XIEventMask},
        xproto::{
            self, ClientMessageEvent, ConnectionExt as _, CreateWindowAux, EventMask, WindowClass,
        },
        Event,
    },
    rust_connection::RustConnection,
    COPY_DEPTH_FROM_PARENT, COPY_FROM_PARENT,
};
use zaxis::{
    vec2,
    winit::{
        event::{ElementState, MouseButton},
        keyboard::{ModifiersState, PhysicalKey},
    },
    InputEvent, KeyInput, WheelDelta,
};

/// The reader thread of one window. Dropping it stops the thread.
pub(super) struct X11Input {
    stop: Arc<AtomicBool>,
    wake: Arc<Wake>,
    thread: Option<JoinHandle<()>>,
}

/// What it takes to wake the reader from outside: its connection and a window of its own to
/// send a message to.
struct Wake {
    connection: std::sync::Mutex<Option<(Arc<RustConnection>, u32)>>,
}

impl X11Input {
    /// Start reading the input of `window`, which belongs to the X server of `$DISPLAY`.
    pub fn start(core: Arc<Core>, window: u32) -> Option<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let wake = Arc::new(Wake {
            connection: std::sync::Mutex::new(None),
        });
        let thread = std::thread::Builder::new()
            .name("z-hook-x11".into())
            .spawn({
                let (stop, wake) = (Arc::clone(&stop), Arc::clone(&wake));
                move || {
                    if let Err(error) = read(&core, window, &stop, &wake) {
                        log::warn!("z-hook: no X11 input for window {window:#x}: {error}");
                    }
                }
            })
            .ok()?;
        Some(Self {
            stop,
            wake,
            thread: Some(thread),
        })
    }
}

impl Drop for X11Input {
    fn drop(&mut self) {
        self.stop.store(true, Relaxed);
        if let Some((connection, helper)) = self.wake.connection.lock().ok().and_then(|c| c.clone())
        {
            let event = ClientMessageEvent::new(32, helper, xproto::AtomEnum::STRING, [0u32; 5]);
            let _ = connection.send_event(false, helper, EventMask::NO_EVENT, event);
            let _ = connection.flush();
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn read(
    core: &Arc<Core>,
    window: u32,
    stop: &AtomicBool,
    wake: &Wake,
) -> Result<(), Box<dyn std::error::Error>> {
    let (connection, screen) = RustConnection::connect(None)?;
    let connection = Arc::new(connection);
    let root = connection.setup().roots[screen].root;
    xinput_version(&connection)?;
    let helper = connection.generate_id()?;
    connection.create_window(
        COPY_DEPTH_FROM_PARENT,
        helper,
        root,
        0,
        0,
        1,
        1,
        0,
        WindowClass::INPUT_ONLY,
        COPY_FROM_PARENT,
        &CreateWindowAux::new(),
    )?;
    *wake.connection.lock().unwrap_or_else(|p| p.into_inner()) =
        Some((Arc::clone(&connection), helper));
    let mask = XIEventMask::KEY_PRESS
        | XIEventMask::KEY_RELEASE
        | XIEventMask::BUTTON_PRESS
        | XIEventMask::BUTTON_RELEASE
        | XIEventMask::MOTION
        | XIEventMask::ENTER
        | XIEventMask::LEAVE
        | XIEventMask::FOCUS_IN
        | XIEventMask::FOCUS_OUT;
    // 1 is `XIAllMasterDevices`.
    connection
        .xinput_xi_select_events(
            window,
            &[xinput::EventMask {
                deviceid: 1,
                mask: vec![mask],
            }],
        )?
        .check()?;
    let setup = connection.setup();
    let (first, last) = (setup.min_keycode, setup.max_keycode);
    let mapping = connection
        .get_keyboard_mapping(first, last - first + 1)?
        .reply()?;
    let keymap = KeyMap {
        first_keycode: first,
        per_keycode: usize::from(mapping.keysyms_per_keycode),
        keysyms: mapping.keysyms,
    };
    log::info!("z-hook: reading X11 input of window {window:#x}");
    while !stop.load(Relaxed) && !core.is_disabled() {
        let event = connection.wait_for_event()?;
        if stop.load(Relaxed) {
            break;
        }
        log::trace!("z-hook: X11 event {event:?}");
        for event in translate(&event, &keymap) {
            core.route(event);
        }
    }
    let _ = connection.destroy_window(helper);
    Ok(())
}

fn xinput_version(connection: &RustConnection) -> Result<(), Box<dyn std::error::Error>> {
    let version = connection.xinput_xi_query_version(2, 0)?.reply()?;
    if version.major_version < 2 {
        return Err("the X server has no XInput 2".into());
    }
    Ok(())
}

fn modifiers(effective: u32) -> ModifiersState {
    let mut state = ModifiersState::empty();
    state.set(ModifiersState::SHIFT, effective & 1 != 0);
    state.set(ModifiersState::CONTROL, effective & 4 != 0);
    state.set(ModifiersState::ALT, effective & 8 != 0);
    state.set(ModifiersState::SUPER, effective & 64 != 0);
    state
}

/// 16.16 fixed point to a number of pixels.
fn pixels(value: i32) -> f64 {
    f64::from(value) / 65536.0
}

/// `XIKeyRepeat`: the press is the server's auto-repeat.
const KEY_REPEAT: u32 = 1 << 16;

fn translate(event: &Event, keymap: &KeyMap) -> Vec<InputEvent> {
    match event {
        Event::XinputMotion(e) => vec![InputEvent::PointerMoved {
            x: pixels(e.event_x),
            y: pixels(e.event_y),
        }],
        Event::XinputButtonPress(e) | Event::XinputButtonRelease(e) => {
            let pressed = matches!(event, Event::XinputButtonPress(_));
            let at = InputEvent::PointerMoved {
                x: pixels(e.event_x),
                y: pixels(e.event_y),
            };
            let state = if pressed {
                ElementState::Pressed
            } else {
                ElementState::Released
            };
            let button = match e.detail {
                1 => MouseButton::Left,
                2 => MouseButton::Middle,
                3 => MouseButton::Right,
                8 => MouseButton::Back,
                9 => MouseButton::Forward,
                4..=7 => {
                    // Wheel steps are presses of buttons 4 to 7; their releases mean nothing.
                    let step = match e.detail {
                        4 => vec2(0.0, 1.0),
                        5 => vec2(0.0, -1.0),
                        6 => vec2(-1.0, 0.0),
                        _ => vec2(1.0, 0.0),
                    };
                    return if pressed {
                        vec![at, InputEvent::Wheel(WheelDelta::Lines(step))]
                    } else {
                        Vec::new()
                    };
                }
                other => MouseButton::Other(other as u16),
            };
            vec![
                InputEvent::Modifiers(modifiers(e.mods.effective)),
                at,
                InputEvent::Button { button, state },
            ]
        }
        Event::XinputKeyPress(e) | Event::XinputKeyRelease(e) => {
            let pressed = matches!(event, Event::XinputKeyPress(_));
            let mods = e.mods.effective;
            let keysym = keymap.keysym(
                e.detail,
                e.group.effective,
                mods & 1 != 0,
                mods & 2 != 0,
                mods & 16 != 0,
            );
            let text = keys::keysym_char(keysym)
                .filter(|_| mods & (4 | 8 | 64) == 0)
                .map(String::from);
            let physical = keys::physical_key(e.detail).map_or(
                PhysicalKey::Unidentified(zaxis::winit::keyboard::NativeKeyCode::Xkb(e.detail)),
                PhysicalKey::Code,
            );
            vec![
                InputEvent::Modifiers(modifiers(mods)),
                InputEvent::Key(KeyInput {
                    physical,
                    logical: keys::logical_key(keysym, text.as_deref()),
                    state: if pressed {
                        ElementState::Pressed
                    } else {
                        ElementState::Released
                    },
                    repeat: pressed && u32::from(e.flags) & KEY_REPEAT != 0,
                    text: text.filter(|_| pressed).map(|t| t.as_str().into()),
                }),
            ]
        }
        Event::XinputEnter(e) => vec![InputEvent::PointerMoved {
            x: pixels(e.event_x),
            y: pixels(e.event_y),
        }],
        Event::XinputLeave(_) => vec![InputEvent::PointerLeft],
        Event::XinputFocusIn(_) => vec![InputEvent::Focus(true)],
        Event::XinputFocusOut(_) => vec![InputEvent::Focus(false)],
        _ => Vec::new(),
    }
}
