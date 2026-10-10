//! A minimal Direct3D 11 application, a triangle in a window, to run the overlay against.
//!
//! `d3d11_host [--overlay path\to\dxgi_overlay.dll]`: with the flag, the host loads the library
//! itself and starts the overlay. Windows only.

#[cfg(not(windows))]
fn main() {
    eprintln!("d3d11_host runs on Windows only");
}

#[cfg(windows)]
fn main() {
    host::run();
}

#[cfg(windows)]
mod host {
    use windows::{
        core::{s, w, Interface, PCSTR, PCWSTR},
        Win32::{
            Foundation::*,
            Graphics::{
                Direct3D::{Fxc::D3DCompile, *},
                Direct3D11::*,
                Dxgi::{Common::*, *},
            },
            System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
            UI::WindowsAndMessaging::*,
        },
    };

    const SHADER: &str = "
        float4 vs(uint id : SV_VertexID) : SV_Position {
            float2 p[3] = { float2(0.0, 0.6), float2(0.6, -0.6), float2(-0.6, -0.6) };
            return float4(p[id], 0.0, 1.0);
        }
        float4 ps() : SV_Target { return float4(0.9, 0.5, 0.1, 1.0); }";

    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: the system's own functions with the window's arguments.
        unsafe {
            if msg == WM_DESTROY {
                PostQuitMessage(0);
                return LRESULT(0);
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }

    fn compile(entry: PCSTR, target: PCSTR) -> ID3DBlob {
        let mut blob = None;
        // SAFETY: valid source text and out pointer.
        unsafe {
            D3DCompile(
                SHADER.as_ptr().cast(),
                SHADER.len(),
                None,
                None,
                None,
                entry,
                target,
                0,
                0,
                &mut blob,
                None,
            )
        }
        .expect("the shader compiles");
        blob.expect("a blob")
    }

    pub fn run() {
        // SAFETY: plain Win32 and Direct3D 11 use; objects live until the process ends.
        unsafe {
            let overlay = std::env::args().skip_while(|a| a != "--overlay").nth(1);
            let instance = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: w!("d3d11_host"),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
                ..Default::default()
            };
            RegisterClassW(&class);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("d3d11_host"),
                w!("d3d11 host"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                960,
                600,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .unwrap();
            let desc = DXGI_SWAP_CHAIN_DESC {
                BufferDesc: DXGI_MODE_DESC {
                    Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                    ..Default::default()
                },
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: 2,
                OutputWindow: hwnd,
                Windowed: true.into(),
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                ..Default::default()
            };
            let (mut swapchain, mut device, mut context) = (None, None, None);
            D3D11CreateDeviceAndSwapChain(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&desc),
                Some(&mut swapchain),
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .expect("a device");
            let (swapchain, device, context) =
                (swapchain.unwrap(), device.unwrap(), context.unwrap());
            if let Some(path) = overlay {
                let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
                let library =
                    LoadLibraryW(PCWSTR(wide.as_ptr())).expect("the overlay library loads");
                let start = GetProcAddress(library, s!("zaxis_overlay_start"))
                    .expect("zaxis_overlay_start");
                let start: extern "C" fn() -> i32 = std::mem::transmute(start);
                println!("overlay start: {}", start());
            }
            let (vs, ps) = (
                compile(s!("vs"), s!("vs_4_0")),
                compile(s!("ps"), s!("ps_4_0")),
            );
            let bytes = |b: &ID3DBlob| {
                std::slice::from_raw_parts(b.GetBufferPointer().cast::<u8>(), b.GetBufferSize())
            };
            let (mut vertex, mut pixel) = (None, None);
            device
                .CreateVertexShader(bytes(&vs), None, Some(&mut vertex))
                .unwrap();
            device
                .CreatePixelShader(bytes(&ps), None, Some(&mut pixel))
                .unwrap();
            let mut message = MSG::default();
            loop {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    if message.message == WM_QUIT {
                        return;
                    }
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let buffer: ID3D11Texture2D = swapchain.GetBuffer(0).unwrap();
                let mut target = None;
                device
                    .CreateRenderTargetView(&buffer, None, Some(&mut target))
                    .unwrap();
                let target = target.unwrap();
                let mut size = RECT::default();
                let _ = GetClientRect(hwnd, &mut size);
                context.OMSetRenderTargets(Some(&[Some(target.clone())]), None);
                context.RSSetViewports(Some(&[D3D11_VIEWPORT {
                    Width: (size.right - size.left) as f32,
                    Height: (size.bottom - size.top) as f32,
                    MaxDepth: 1.0,
                    ..Default::default()
                }]));
                context.ClearRenderTargetView(&target, &[0.1, 0.2, 0.3, 1.0]);
                context.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
                context.VSSetShader(vertex.as_ref(), None);
                context.PSSetShader(pixel.as_ref(), None);
                context.Draw(3, 0);
                let _ = swapchain.Present(1, DXGI_PRESENT(0));
                let _ = swapchain.cast::<IDXGISwapChain1>();
            }
        }
    }
}
