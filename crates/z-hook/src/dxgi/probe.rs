//! Finding the swapchain vtable: make a swapchain of our own on a window nobody sees, and read
//! the slots of its class. Every swapchain DXGI makes (Direct3D 11 or 12) is of that class.

use std::ffi::c_void;
use windows::{
    core::{w, Interface},
    Win32::{
        Foundation::{HMODULE, HWND},
        Graphics::{
            Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP},
            Direct3D11::{
                D3D11CreateDeviceAndSwapChain, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
            },
            Dxgi::{Common::*, *},
        },
        UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_OVERLAPPED},
    },
};

/// The addresses of the slots to patch.
pub(super) struct Slots {
    pub present: *const c_void,
    pub resize_buffers: *const c_void,
    pub present1: Option<*const c_void>,
    pub resize_buffers1: Option<*const c_void>,
}

pub(super) fn probe() -> Result<Slots, String> {
    // SAFETY: plain window and device creation; everything made here is released below.
    unsafe {
        let window = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("STATIC"),
            w!("z-hook probe"),
            WS_OVERLAPPED,
            0,
            0,
            8,
            8,
            None,
            None,
            None,
            None,
        )
        .map_err(|e| format!("probe window: {e}"))?;
        let result = slots_of(window);
        let _ = DestroyWindow(window);
        result
    }
}

unsafe fn slots_of(window: HWND) -> Result<Slots, String> {
    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Width: 8,
            Height: 8,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ..Default::default()
        },
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        OutputWindow: window,
        Windowed: true.into(),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        ..Default::default()
    };
    let mut swapchain = None;
    let mut last = String::new();
    for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        // SAFETY: valid descriptor and out pointers.
        let made = unsafe {
            D3D11CreateDeviceAndSwapChain(
                None,
                driver,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&desc),
                Some(&mut swapchain),
                None,
                None,
                None,
            )
        };
        match made {
            Ok(()) if swapchain.is_some() => break,
            Ok(()) => last = "no swapchain".into(),
            Err(error) => last = error.to_string(),
        }
    }
    let swapchain: IDXGISwapChain = swapchain.ok_or_else(|| format!("probe swapchain: {last}"))?;
    let base = swapchain.vtable();
    let mut slots = Slots {
        present: std::ptr::from_ref(&base.Present).cast(),
        resize_buffers: std::ptr::from_ref(&base.ResizeBuffers).cast(),
        present1: None,
        resize_buffers1: None,
    };
    if let Ok(sc1) = swapchain.cast::<IDXGISwapChain1>() {
        slots.present1 = Some(std::ptr::from_ref(&sc1.vtable().Present1).cast());
    }
    if let Ok(sc3) = swapchain.cast::<IDXGISwapChain3>() {
        slots.resize_buffers1 = Some(std::ptr::from_ref(&sc3.vtable().ResizeBuffers1).cast());
    }
    // The vtables are static data of the DXGI library: the slots stay valid after the probe
    // objects are released.
    Ok(slots)
}
