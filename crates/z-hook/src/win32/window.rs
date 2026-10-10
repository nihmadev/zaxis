//! The host's window procedure, replaced by one that reads its messages.
//!
//! Each message goes through the [`Translator`]; its events go to the overlay, and
//! [`host_gets`] decides from the answers whether the host's procedure sees the message.
//! While the overlay claims the pointer, the cursor the host hid or confined is released
//! and the interface's cursor shown; everything is put back when it lets go.
//!
//! **Not run on Windows.** Written against the Win32 documentation; it compiles for
//! `x86_64-pc-windows-msvc`.

use super::{host_gets, ime_composition, Class, Situation, Translator};
use crate::{core::Core, input::Delivery, registry};
use std::{
    collections::HashMap,
    sync::{
        atomic::{
            AtomicBool, AtomicIsize,
            Ordering::{Relaxed, SeqCst},
        },
        Arc, Mutex, PoisonError,
    },
};
use windows::{
    core::PCWSTR,
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
        UI::{
            Input::{
                Ime::*,
                KeyboardAndMouse::{
                    GetKeyState, TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT, VK_CONTROL, VK_LWIN,
                    VK_MENU, VK_RWIN, VK_SHIFT,
                },
            },
            WindowsAndMessaging::*,
        },
    },
};
use zaxis::winit::keyboard::ModifiersState;

struct Hooked {
    original: isize,
    translator: Mutex<Translator>,
    tracking: AtomicBool,
}

static WINDOWS: Mutex<Option<HashMap<isize, Arc<Hooked>>>> = Mutex::new(None);

/// What the cursor looked like before the overlay took it over.
struct Saved {
    clip: RECT,
    shown_by_us: i32,
}

static SAVED: Mutex<Option<Saved>> = Mutex::new(None);
static OUR_PROC: AtomicIsize = AtomicIsize::new(0);

/// Replace the window procedure of `hwnd`. Returns whether the window is hooked.
pub(crate) fn attach(hwnd: HWND) -> bool {
    let key = hwnd.0 as isize;
    let mut windows = WINDOWS.lock().unwrap_or_else(PoisonError::into_inner);
    let windows = windows.get_or_insert_with(HashMap::new);
    if windows.contains_key(&key) {
        return true;
    }
    let ours = window_proc as *const () as isize;
    OUR_PROC.store(ours, SeqCst);
    // SAFETY: replacing the procedure of a window of this process; the old one is called
    // through `CallWindowProcW` for everything the overlay does not take.
    let original = unsafe { SetWindowLongPtrW(hwnd, GWLP_WNDPROC, ours) };
    if original == 0 {
        log::warn!(
            "z-hook: cannot hook the window procedure; the overlay gets no input from this window"
        );
        return false;
    }
    let mut translator = Translator::new();
    translator.set_modifiers(current_modifiers());
    windows.insert(
        key,
        Arc::new(Hooked {
            original,
            translator: Mutex::new(translator),
            tracking: AtomicBool::new(false),
        }),
    );
    log::info!("z-hook: reading the messages of window {key:#x}");
    true
}

/// Give every window its procedure back where nothing replaced ours since, and the cursor its state.
pub(crate) fn detach_all() {
    let windows = WINDOWS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take()
        .unwrap_or_default();
    let ours = OUR_PROC.load(SeqCst);
    for (key, hooked) in windows {
        let hwnd = HWND(key as *mut _);
        // SAFETY: plain queries and a restore of the procedure we replaced.
        unsafe {
            if GetWindowLongPtrW(hwnd, GWLP_WNDPROC) == ours {
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, hooked.original);
            } else {
                log::warn!("z-hook: window {key:#x} was subclassed after the overlay; its procedure stays and forwards");
            }
        }
    }
    release_cursor();
}

fn current_modifiers() -> ModifiersState {
    let down = |vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY| {
        // SAFETY: a plain query.
        let state = unsafe { GetKeyState(i32::from(vk.0)) };
        state < 0
    };
    let mut state = ModifiersState::empty();
    state.set(ModifiersState::SHIFT, down(VK_SHIFT));
    state.set(ModifiersState::CONTROL, down(VK_CONTROL));
    state.set(ModifiersState::ALT, down(VK_MENU));
    state.set(ModifiersState::SUPER, down(VK_LWIN) || down(VK_RWIN));
    state
}

fn situation(core: &Core) -> Situation {
    let policy = &core.hub.policy;
    Situation {
        mode: core.options.input,
        visible: core.is_visible(),
        pointer_over_ui: policy.pointer_over_ui.load(Relaxed),
        keyboard_focus: policy.keyboard_focus.load(Relaxed),
    }
}

/// Show the cursor and free it from the host's confinement while the overlay claims the
/// mouse in a mode that takes it from the host; undo it when it stops.
fn sync_cursor(core: &Core) {
    let wants = core.is_visible() && core.options.input == crate::OverlayInput::CaptureWhenVisible;
    let mut saved = SAVED.lock().unwrap_or_else(PoisonError::into_inner);
    match (wants, saved.is_some()) {
        (true, false) => {
            let mut clip = RECT::default();
            // SAFETY: plain queries and changes of the process's cursor state, on the thread
            // that owns the window (the display counter belongs to the thread).
            unsafe {
                let _ = GetClipCursor(&mut clip);
                let _ = ClipCursor(None);
                let mut shown_by_us = 0;
                let mut count = ShowCursor(true);
                while count < 0 && shown_by_us < 32 {
                    shown_by_us += 1;
                    count = ShowCursor(true);
                }
                if count >= 0 && shown_by_us == 0 {
                    // Already visible: the matching `ShowCursor(true)` above must be undone too.
                    shown_by_us = 1;
                }
                *saved = Some(Saved { clip, shown_by_us });
            }
        }
        (false, true) => release(saved.take()),
        _ => {}
    }
}

fn release_cursor() {
    release(SAVED.lock().unwrap_or_else(PoisonError::into_inner).take());
}

fn release(saved: Option<Saved>) {
    let Some(saved) = saved else { return };
    // SAFETY: undoes what `sync_cursor` did.
    unsafe {
        for _ in 0..saved.shown_by_us {
            ShowCursor(false);
        }
        let _ = ClipCursor(Some(&saved.clip));
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let hooked = WINDOWS
        .lock()
        .ok()
        .and_then(|w| w.as_ref().and_then(|w| w.get(&(hwnd.0 as isize)).cloned()));
    let Some(hooked) = hooked else {
        // SAFETY: the system's default procedure; the window was never or is no longer hooked.
        return unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) };
    };
    let mut forward = true;
    let mut result = None;
    crate::driver::guard_frame("a window message", || {
        let Some(core) = registry::current() else {
            return;
        };
        sync_cursor(&core);
        (forward, result) = handle(&core, &hooked, hwnd, msg, wparam, lparam);
    });
    if msg == WM_NCDESTROY {
        if let Ok(mut windows) = WINDOWS.lock() {
            if let Some(windows) = windows.as_mut() {
                windows.remove(&(hwnd.0 as isize));
            }
        }
    }
    match (forward, result) {
        (_, Some(result)) => result,
        // SAFETY: the window's procedure before ours, with the message unchanged.
        (true, None) => unsafe {
            CallWindowProcW(
                std::mem::transmute::<isize, WNDPROC>(hooked.original),
                hwnd,
                msg,
                wparam,
                lparam,
            )
        },
        (false, None) => LRESULT(0),
    }
}

/// Route a message. `(whether the host's procedure gets it, an answer of our own)`.
fn handle(
    core: &Arc<Core>,
    hooked: &Hooked,
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> (bool, Option<LRESULT>) {
    if msg == WM_MOUSEMOVE && !hooked.tracking.swap(true, Relaxed) {
        let mut track = TRACKMOUSEEVENT {
            cbSize: size_of::<TRACKMOUSEEVENT>() as u32,
            dwFlags: TME_LEAVE,
            hwndTrack: hwnd,
            dwHoverTime: 0,
        };
        // SAFETY: a valid structure for this window.
        let _ = unsafe { TrackMouseEvent(&mut track) };
    }
    if msg == super::wm::MOUSELEAVE {
        hooked.tracking.store(false, Relaxed);
    }
    let mut translation = hooked
        .translator
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .translate(msg, wparam.0, lparam.0);
    if translation.class == Class::Ime {
        translation.events = ime_events(core, hwnd, msg, lparam.0);
    }
    let deliveries: Vec<Delivery> = translation
        .events
        .drain(..)
        .map(|event| core.route(event))
        .collect();
    let situation = situation(core);
    if translation.class == Class::Cursor
        && situation.takes_pointer()
        && (lparam.0 & 0xFFFF) as u32 == HTCLIENT
    {
        let idc = core.hub.policy.cursor.load(Relaxed);
        // SAFETY: loads a stock cursor; the resource id is one of the `IDC_*` values.
        let cursor = unsafe { LoadCursorW(None, PCWSTR(idc as usize as *const u16)) };
        if let Ok(cursor) = cursor {
            // SAFETY: a valid cursor handle.
            unsafe { SetCursor(Some(cursor)) };
            return (false, Some(LRESULT(1)));
        }
    }
    (host_gets(translation.class, &deliveries, situation), None)
}

/// The events of an input method message, from the composition the input method holds.
fn ime_events(core: &Core, hwnd: HWND, msg: u32, lparam: isize) -> Vec<zaxis::InputEvent> {
    use super::wm;
    if msg == wm::IME_ENDCOMPOSITION {
        return vec![zaxis::InputEvent::Ime(zaxis::ImeEvent::Preedit(
            String::new(),
            None,
        ))];
    }
    // SAFETY: the context belongs to this window and is released before returning.
    unsafe {
        let himc = ImmGetContext(hwnd);
        if himc.is_invalid() {
            return Vec::new();
        }
        if let Some([x, y, _, h]) = *crate::core::lock(&core.hub.policy.ime_area) {
            let form = COMPOSITIONFORM {
                dwStyle: CFS_POINT,
                ptCurrentPos: windows::Win32::Foundation::POINT {
                    x: x as i32,
                    y: (y + h) as i32,
                },
                rcArea: RECT::default(),
            };
            let _ = ImmSetCompositionWindow(himc, &form);
        }
        let string = |flag: IME_COMPOSITION_STRING| -> Option<String> {
            let bytes = ImmGetCompositionStringW(himc, flag, None, 0);
            if bytes <= 0 {
                return None;
            }
            let mut buffer = vec![0u16; bytes as usize / 2];
            ImmGetCompositionStringW(himc, flag, Some(buffer.as_mut_ptr().cast()), bytes as u32);
            Some(String::from_utf16_lossy(&buffer))
        };
        let events = if msg == wm::IME_COMPOSITION {
            let result = (lparam as u32 & GCS_RESULTSTR.0 != 0)
                .then(|| string(GCS_RESULTSTR))
                .flatten();
            let composing = (lparam as u32 & GCS_COMPSTR.0 != 0)
                .then(|| string(GCS_COMPSTR).unwrap_or_default())
                .map(|text| {
                    let caret =
                        ImmGetCompositionStringW(himc, GCS_CURSORPOS, None, 0).max(0) as usize;
                    let offset = text
                        .char_indices()
                        .nth(caret)
                        .map_or(text.len(), |(i, _)| i);
                    (text, Some((offset, offset)))
                });
            ime_composition(
                result.as_deref(),
                composing.as_ref().map(|(t, c)| (t.as_str(), *c)),
            )
        } else {
            Vec::new()
        };
        let _ = ImmReleaseContext(hwnd, himc);
        events
    }
}
