//! Replacing one slot of a COM vtable.
//!
//! A vtable is shared by every object of its class, so one patched slot covers every
//! swapchain the host makes. Overwriting a pointer needs no trampoline and no instruction
//! decoding (the reason this is not an inline detour); the original function is called through
//! the pointer that was in the slot. The cost is that it only sees calls made through the vtable.

use std::ffi::c_void;
use windows::Win32::System::Memory::{VirtualProtect, PAGE_PROTECTION_FLAGS, PAGE_READWRITE};

/// A patched slot.
pub(super) struct Patch {
    slot: *mut usize,
    pub original: usize,
    hook: usize,
}

// SAFETY: a patch is a pair of addresses; the memory they name is process-global and only
// written through `patch` and `restore`.
unsafe impl Send for Patch {}
unsafe impl Sync for Patch {}

/// Put `hook` in `slot`, returning the patch that remembers what was there.
///
/// # Safety
/// `slot` must be a slot of a live vtable and `hook` a function with that slot's signature.
pub(super) unsafe fn patch(slot: *const c_void, hook: usize) -> Result<Patch, String> {
    let slot = slot.cast::<usize>().cast_mut();
    let mut previous = PAGE_PROTECTION_FLAGS(0);
    // SAFETY: the slot is one pointer in mapped memory.
    unsafe {
        VirtualProtect(
            slot.cast(),
            size_of::<usize>(),
            PAGE_READWRITE,
            &mut previous,
        )
    }
    .map_err(|error| format!("VirtualProtect: {error}"))?;
    // SAFETY: the page is writable now; one aligned pointer is replaced atomically enough for
    // a reader on another thread to see the old or the new function.
    let original = unsafe { slot.read_volatile() };
    unsafe { slot.write_volatile(hook) };
    let mut ignored = PAGE_PROTECTION_FLAGS(0);
    // SAFETY: restores the protection that `VirtualProtect` reported.
    let _ = unsafe { VirtualProtect(slot.cast(), size_of::<usize>(), previous, &mut ignored) };
    Ok(Patch {
        slot,
        original,
        hook,
    })
}

impl Patch {
    /// Put the original back, unless someone else patched the slot after us (then our hook
    /// stays in place and forwards, and the caller keeps it alive).
    ///
    /// # Safety
    /// The vtable must still be mapped.
    pub unsafe fn restore(&self) -> bool {
        // SAFETY: as in `patch`.
        unsafe {
            if self.slot.read_volatile() != self.hook {
                return false;
            }
            let mut previous = PAGE_PROTECTION_FLAGS(0);
            if VirtualProtect(
                self.slot.cast(),
                size_of::<usize>(),
                PAGE_READWRITE,
                &mut previous,
            )
            .is_err()
            {
                return false;
            }
            self.slot.write_volatile(self.original);
            let mut ignored = PAGE_PROTECTION_FLAGS(0);
            let _ = VirtualProtect(self.slot.cast(), size_of::<usize>(), previous, &mut ignored);
        }
        true
    }
}
