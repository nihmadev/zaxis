//! A minimal Direct3D 12 application, a triangle in a window, to run the overlay against.
//!
//! `d3d12_host [--overlay path\to\dxgi_overlay.dll]`. Windows only.

#[cfg(not(windows))]
fn main() {
    eprintln!("d3d12_host runs on Windows only");
}

#[cfg(windows)]
fn main() {
    host::run();
}

#[cfg(windows)]
mod host {
    use std::mem::ManuallyDrop;
    use windows::{
        core::{s, w, Interface, PCSTR, PCWSTR},
        Win32::{
            Foundation::*,
            Graphics::{
                Direct3D::{Fxc::D3DCompile, *},
                Direct3D12::*,
                Dxgi::{Common::*, *},
            },
            System::{
                LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW},
                Threading::{CreateEventW, WaitForSingleObject, INFINITE},
            },
            UI::WindowsAndMessaging::*,
        },
    };

    const FRAMES: usize = 3;
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

    fn barrier(
        resource: &ID3D12Resource,
        before: D3D12_RESOURCE_STATES,
        after: D3D12_RESOURCE_STATES,
    ) -> D3D12_RESOURCE_BARRIER {
        D3D12_RESOURCE_BARRIER {
            Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
            Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
            Anonymous: D3D12_RESOURCE_BARRIER_0 {
                Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                    // SAFETY: a borrowed pointer, never released by `ManuallyDrop`.
                    pResource: unsafe { std::mem::transmute_copy(resource) },
                    Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                    StateBefore: before,
                    StateAfter: after,
                }),
            },
        }
    }

    pub fn run() {
        // SAFETY: plain Win32 and Direct3D 12 use; objects live until the process ends.
        unsafe {
            let overlay = std::env::args().skip_while(|a| a != "--overlay").nth(1);
            let instance = GetModuleHandleW(None).unwrap();
            let class = WNDCLASSW {
                lpfnWndProc: Some(proc),
                hInstance: instance.into(),
                lpszClassName: w!("d3d12_host"),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap(),
                ..Default::default()
            };
            RegisterClassW(&class);
            let (width, height) = (960u32, 600u32);
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("d3d12_host"),
                w!("d3d12 host"),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                100,
                100,
                width as i32,
                height as i32,
                None,
                None,
                Some(instance.into()),
                None,
            )
            .unwrap();
            let factory: IDXGIFactory4 = CreateDXGIFactory1().unwrap();
            let mut device: Option<ID3D12Device> = None;
            D3D12CreateDevice(None, D3D_FEATURE_LEVEL_11_0, &mut device)
                .expect("a Direct3D 12 device");
            let device = device.unwrap();
            let queue: ID3D12CommandQueue = device
                .CreateCommandQueue(&D3D12_COMMAND_QUEUE_DESC {
                    Type: D3D12_COMMAND_LIST_TYPE_DIRECT,
                    ..Default::default()
                })
                .unwrap();
            let desc = DXGI_SWAP_CHAIN_DESC1 {
                Width: width,
                Height: height,
                Format: DXGI_FORMAT_R8G8B8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                BufferCount: FRAMES as u32,
                SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                ..Default::default()
            };
            let swapchain: IDXGISwapChain3 = factory
                .CreateSwapChainForHwnd(&queue, hwnd, &desc, None, None)
                .unwrap()
                .cast()
                .unwrap();
            if let Some(path) = overlay {
                let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
                let library =
                    LoadLibraryW(PCWSTR(wide.as_ptr())).expect("the overlay library loads");
                let start = GetProcAddress(library, s!("zaxis_overlay_start"))
                    .expect("zaxis_overlay_start");
                let start: extern "C" fn() -> i32 = std::mem::transmute(start);
                println!("overlay start: {}", start());
            }
            let heap: ID3D12DescriptorHeap = device
                .CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                    Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
                    NumDescriptors: FRAMES as u32,
                    ..Default::default()
                })
                .unwrap();
            let step =
                device.GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV) as usize;
            let start = heap.GetCPUDescriptorHandleForHeapStart();
            let buffers: Vec<ID3D12Resource> = (0..FRAMES)
                .map(|i| {
                    let buffer: ID3D12Resource = swapchain.GetBuffer(i as u32).unwrap();
                    device.CreateRenderTargetView(
                        &buffer,
                        None,
                        D3D12_CPU_DESCRIPTOR_HANDLE {
                            ptr: start.ptr + i * step,
                        },
                    );
                    buffer
                })
                .collect();
            let mut blob = None;
            D3D12SerializeRootSignature(
                &D3D12_ROOT_SIGNATURE_DESC::default(),
                D3D_ROOT_SIGNATURE_VERSION_1,
                &mut blob,
                None,
            )
            .unwrap();
            let blob: ID3DBlob = blob.unwrap();
            let signature: ID3D12RootSignature = device
                .CreateRootSignature(
                    0,
                    std::slice::from_raw_parts(
                        blob.GetBufferPointer().cast(),
                        blob.GetBufferSize(),
                    ),
                )
                .unwrap();
            let (vs, ps) = (
                compile(s!("vs"), s!("vs_5_0")),
                compile(s!("ps"), s!("ps_5_0")),
            );
            let shader = |b: &ID3DBlob| D3D12_SHADER_BYTECODE {
                pShaderBytecode: b.GetBufferPointer(),
                BytecodeLength: b.GetBufferSize(),
            };
            let mut pso_desc = D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: ManuallyDrop::new(Some(signature.clone())),
                VS: shader(&vs),
                PS: shader(&ps),
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    DepthClipEnable: true.into(),
                    ..Default::default()
                },
                BlendState: D3D12_BLEND_DESC {
                    RenderTarget: [D3D12_RENDER_TARGET_BLEND_DESC {
                        RenderTargetWriteMask: 15,
                        ..Default::default()
                    }; 8],
                    ..Default::default()
                },
                SampleMask: u32::MAX,
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                NumRenderTargets: 1,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                ..Default::default()
            };
            pso_desc.RTVFormats[0] = DXGI_FORMAT_R8G8B8A8_UNORM;
            let pipeline: ID3D12PipelineState =
                device.CreateGraphicsPipelineState(&pso_desc).unwrap();
            let allocators: Vec<ID3D12CommandAllocator> = (0..FRAMES)
                .map(|_| {
                    device
                        .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT)
                        .unwrap()
                })
                .collect();
            let list: ID3D12GraphicsCommandList = device
                .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocators[0], &pipeline)
                .unwrap();
            list.Close().unwrap();
            let fence: ID3D12Fence = device.CreateFence(0, D3D12_FENCE_FLAG_NONE).unwrap();
            let event = CreateEventW(None, false, false, None).unwrap();
            let mut values = [0u64; FRAMES];
            let mut counter = 0u64;
            let mut message = MSG::default();
            loop {
                while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                    if message.message == WM_QUIT {
                        return;
                    }
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let index = swapchain.GetCurrentBackBufferIndex() as usize;
                if fence.GetCompletedValue() < values[index] {
                    fence.SetEventOnCompletion(values[index], event).unwrap();
                    WaitForSingleObject(event, INFINITE);
                }
                allocators[index].Reset().unwrap();
                list.Reset(&allocators[index], &pipeline).unwrap();
                let target = D3D12_CPU_DESCRIPTOR_HANDLE {
                    ptr: start.ptr + index * step,
                };
                list.SetGraphicsRootSignature(&signature);
                list.RSSetViewports(&[D3D12_VIEWPORT {
                    Width: width as f32,
                    Height: height as f32,
                    MaxDepth: 1.0,
                    ..Default::default()
                }]);
                list.RSSetScissorRects(&[RECT {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                }]);
                list.ResourceBarrier(&[barrier(
                    &buffers[index],
                    D3D12_RESOURCE_STATE_PRESENT,
                    D3D12_RESOURCE_STATE_RENDER_TARGET,
                )]);
                list.OMSetRenderTargets(1, Some(&target), false, None);
                list.ClearRenderTargetView(target, &[0.1, 0.2, 0.3, 1.0], None);
                list.IASetPrimitiveTopology(D3D_PRIMITIVE_TOPOLOGY_TRIANGLELIST);
                list.DrawInstanced(3, 1, 0, 0);
                list.ResourceBarrier(&[barrier(
                    &buffers[index],
                    D3D12_RESOURCE_STATE_RENDER_TARGET,
                    D3D12_RESOURCE_STATE_PRESENT,
                )]);
                list.Close().unwrap();
                queue.ExecuteCommandLists(&[Some(list.cast().unwrap())]);
                let _ = swapchain.Present(1, DXGI_PRESENT(0));
                counter += 1;
                queue.Signal(&fence, counter).unwrap();
                values[index] = counter;
            }
        }
    }
}
