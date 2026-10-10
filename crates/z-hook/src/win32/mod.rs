//! Win32 window messages as zaxis input, without a window.
//!
//! Everything here is plain data translation, so it builds and is tested on every platform;
//! the code that subclasses a window and reads the messages is Windows-only and uses it.

mod keys;
mod messages;
mod route;
mod scan;
#[cfg(windows)]
mod window;
#[cfg(windows)]
pub(crate) use window::{attach, detach_all};

pub use keys::{cursor_resource, logical_key, modifier_of};
pub use messages::{ime_composition, Class, Translation, Translator};
pub use route::{host_gets, Situation};
pub use scan::scan_code_to_key;

/// Window message numbers (`WM_*`) the translator understands.
pub mod wm {
    pub const SETFOCUS: u32 = 0x0007;
    pub const KILLFOCUS: u32 = 0x0008;
    pub const SETCURSOR: u32 = 0x0020;
    pub const KEYDOWN: u32 = 0x0100;
    pub const KEYUP: u32 = 0x0101;
    pub const CHAR: u32 = 0x0102;
    pub const SYSKEYDOWN: u32 = 0x0104;
    pub const SYSKEYUP: u32 = 0x0105;
    pub const SYSCHAR: u32 = 0x0106;
    pub const UNICHAR: u32 = 0x0109;
    pub const INPUT: u32 = 0x00FF;
    pub const MOUSEMOVE: u32 = 0x0200;
    pub const LBUTTONDOWN: u32 = 0x0201;
    pub const LBUTTONUP: u32 = 0x0202;
    pub const LBUTTONDBLCLK: u32 = 0x0203;
    pub const RBUTTONDOWN: u32 = 0x0204;
    pub const RBUTTONUP: u32 = 0x0205;
    pub const RBUTTONDBLCLK: u32 = 0x0206;
    pub const MBUTTONDOWN: u32 = 0x0207;
    pub const MBUTTONUP: u32 = 0x0208;
    pub const MBUTTONDBLCLK: u32 = 0x0209;
    pub const MOUSEWHEEL: u32 = 0x020A;
    pub const XBUTTONDOWN: u32 = 0x020B;
    pub const XBUTTONUP: u32 = 0x020C;
    pub const XBUTTONDBLCLK: u32 = 0x020D;
    pub const MOUSEHWHEEL: u32 = 0x020E;
    pub const MOUSELEAVE: u32 = 0x02A3;
    pub const DPICHANGED: u32 = 0x02E0;
    pub const IME_STARTCOMPOSITION: u32 = 0x010D;
    pub const IME_ENDCOMPOSITION: u32 = 0x010E;
    pub const IME_COMPOSITION: u32 = 0x010F;
    pub const IME_SETCONTEXT: u32 = 0x0281;
    pub const IME_NOTIFY: u32 = 0x0282;
    pub const IME_CHAR: u32 = 0x0286;
}
