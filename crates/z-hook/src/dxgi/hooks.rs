//! The hooked `IDXGISwapChain` methods.

use super::{
    probe, state,
    vtable::{self, Patch},
};
use crate::{driver, registry};
use std::{
    cell::Cell,
    ffi::c_void,
    sync::{
        atomic::{
            AtomicBool, AtomicUsize,
            Ordering::{AcqRel, Acquire, Release},
        },
        Mutex, PoisonError,
    },
    time::{Duration, Instant},
};
use windows::{
    core::{Interface, HRESULT},
    Win32::Graphics::Dxgi::{
        Common::DXGI_FORMAT, IDXGISwapChain, DXGI_PRESENT_PARAMETERS, DXGI_PRESENT_TEST,
    },
};

type Present = unsafe extern "system" fn(*mut c_void, u32, u32) -> HRESULT;
type Present1 =
    unsafe extern "system" fn(*mut c_void, u32, u32, *const DXGI_PRESENT_PARAMETERS) -> HRESULT;
type ResizeBuffers =
    unsafe extern "system" fn(*mut c_void, u32, u32, u32, DXGI_FORMAT, u32) -> HRESULT;
type ResizeBuffers1 = unsafe extern "system" fn(
    *mut c_void,
    u32,
    u32,
    u32,
    DXGI_FORMAT,
    u32,
    *const u32,
    *const *mut c_void,
) -> HRESULT;

// The functions the hooks call on: set once, before any slot is patched, and never cleared (a
// hook may be running when the patches are removed).
static PRESENT: AtomicUsize = AtomicUsize::new(0);
static PRESENT1: AtomicUsize = AtomicUsize::new(0);
static RESIZE: AtomicUsize = AtomicUsize::new(0);
static RESIZE1: AtomicUsize = AtomicUsize::new(0);

/// Hooks currently executing; the removal waits for them.
static IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static ACTIVE: AtomicBool = AtomicBool::new(false);
static PATCHES: Mutex<Vec<Patch>> = Mutex::new(Vec::new());

thread_local! {
    /// `Present` may be implemented through `Present1` (or the reverse): only the outer call draws.
    static NESTED: Cell<bool> = const { Cell::new(false) };
}

struct Flight;

impl Flight {
    fn enter() -> Option<Self> {
        IN_FLIGHT.fetch_add(1, AcqRel);
        if ACTIVE.load(Acquire) {
            Some(Self)
        } else {
            IN_FLIGHT.fetch_sub(1, AcqRel);
            None
        }
    }
}

impl Drop for Flight {
    fn drop(&mut self) {
        IN_FLIGHT.fetch_sub(1, AcqRel);
    }
}

pub(super) fn install() -> Result<(), String> {
    let mut patches = PATCHES.lock().unwrap_or_else(PoisonError::into_inner);
    if !patches.is_empty() {
        return Ok(());
    }
    let slots = probe::probe()?;
    super::wgpu_side::reopen();
    // SAFETY: the slots were read from a live swapchain of the class; each hook has the
    // signature of the slot it goes in.
    unsafe {
        let present = vtable::patch(slots.present, present_hook as *const () as usize)?;
        PRESENT.store(present.original, Release);
        patches.push(present);
        let resize = vtable::patch(
            slots.resize_buffers,
            resize_buffers_hook as *const () as usize,
        )?;
        RESIZE.store(resize.original, Release);
        patches.push(resize);
        if let Some(slot) = slots.present1 {
            let patch = vtable::patch(slot, present1_hook as *const () as usize)?;
            PRESENT1.store(patch.original, Release);
            patches.push(patch);
        }
        if let Some(slot) = slots.resize_buffers1 {
            let patch = vtable::patch(slot, resize_buffers1_hook as *const () as usize)?;
            RESIZE1.store(patch.original, Release);
            patches.push(patch);
        }
    }
    ACTIVE.store(true, Release);
    log::info!("z-hook: hooked {} swapchain methods", patches.len());
    Ok(())
}

/// Put the original functions back and wait for hooks in flight; a slot someone else patched
/// since keeps our hook, which forwards.
pub(super) fn remove() {
    ACTIVE.store(false, Release);
    {
        let mut patches = PATCHES.lock().unwrap_or_else(PoisonError::into_inner);
        for patch in patches.drain(..) {
            // SAFETY: the vtables are static data of a library that is still loaded.
            if !unsafe { patch.restore() } {
                log::warn!("z-hook: a swapchain slot was patched after ours; leaving our forwarding hook in place");
                let _ = patch; // nothing to release: the hook stays in place
            }
        }
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while IN_FLIGHT.load(Acquire) > 0 && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(1));
    }
    crate::win32::detach_all();
    state::clear();
    super::wgpu_side::release();
}

fn draw(this: *mut c_void) {
    if NESTED.with(Cell::get) {
        return;
    }
    NESTED.with(|nested| nested.set(true));
    crate::driver::guard_frame("a swapchain present", || {
        let Some(core) = registry::current() else {
            return;
        };
        // SAFETY: `this` is the swapchain the host is presenting.
        let Some(swapchain) = (unsafe { IDXGISwapChain::from_raw_borrowed(&this) }) else {
            return;
        };
        let output = core.options.output;
        state::with_entry(swapchain, |entry| match entry {
            state::Entry::Ready(state) => {
                let mut backend = state::DxgiBackend::new(state, swapchain, output);
                driver::run_frame(&core, &mut backend);
            }
            state::Entry::Unsupported => {}
        });
    });
    NESTED.with(|nested| nested.set(false));
}

unsafe extern "system" fn present_hook(this: *mut c_void, sync: u32, flags: u32) -> HRESULT {
    let _flight = Flight::enter();
    if _flight.is_some() && flags & DXGI_PRESENT_TEST.0 == 0 {
        draw(this);
    }
    // SAFETY: the original function of the slot, stored before it was patched.
    let original: Present = unsafe { std::mem::transmute(PRESENT.load(Acquire)) };
    // SAFETY: the host's arguments, unchanged.
    unsafe { original(this, sync, flags) }
}

unsafe extern "system" fn present1_hook(
    this: *mut c_void,
    sync: u32,
    flags: u32,
    parameters: *const DXGI_PRESENT_PARAMETERS,
) -> HRESULT {
    let _flight = Flight::enter();
    if _flight.is_some() && flags & DXGI_PRESENT_TEST.0 == 0 {
        draw(this);
    }
    // SAFETY: as in `present_hook`.
    let original: Present1 = unsafe { std::mem::transmute(PRESENT1.load(Acquire)) };
    // SAFETY: the host's arguments, unchanged.
    unsafe { original(this, sync, flags, parameters) }
}

/// `ResizeBuffers` fails while any reference to a backbuffer is held: the overlay holds none
/// across calls, and drops what it made for the old size before the original runs.
unsafe extern "system" fn resize_buffers_hook(
    this: *mut c_void,
    count: u32,
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
    flags: u32,
) -> HRESULT {
    let _flight = Flight::enter();
    forget(this);
    // SAFETY: as in `present_hook`.
    let original: ResizeBuffers = unsafe { std::mem::transmute(RESIZE.load(Acquire)) };
    // SAFETY: the host's arguments, unchanged.
    unsafe { original(this, count, width, height, format, flags) }
}

unsafe extern "system" fn resize_buffers1_hook(
    this: *mut c_void,
    count: u32,
    width: u32,
    height: u32,
    format: DXGI_FORMAT,
    flags: u32,
    nodes: *const u32,
    queues: *const *mut c_void,
) -> HRESULT {
    let _flight = Flight::enter();
    forget(this);
    // SAFETY: as in `present_hook`.
    let original: ResizeBuffers1 = unsafe { std::mem::transmute(RESIZE1.load(Acquire)) };
    // SAFETY: the host's arguments, unchanged.
    unsafe { original(this, count, width, height, format, flags, nodes, queues) }
}

fn forget(this: *mut c_void) {
    // SAFETY: `this` is the swapchain being resized.
    if let Some(swapchain) = unsafe { IDXGISwapChain::from_raw_borrowed(&this) } {
        state::drop_target(swapchain);
    }
}
