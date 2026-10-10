//! What is kept per swapchain, and the backend that draws on one.

use super::{
    host11::Host11,
    host12::Host12,
    shared::SharedTarget,
    wgpu_side::{self, WgpuSide},
};
use crate::{
    backend::{BackendError, PresentBackend, SurfaceInfo},
    dxgi_format::{plan, FormatPlan},
    OutputColor,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND,
        Graphics::{
            Direct3D11::{ID3D11Device, ID3D11Resource, ID3D11Texture2D},
            Direct3D12::{ID3D12CommandQueue, ID3D12Device, ID3D12Resource},
            Dxgi::{IDXGIAdapter, IDXGIDevice, IDXGISwapChain, IDXGISwapChain3},
        },
        UI::HiDpi::GetDpiForWindow,
    },
};
use zaxis::{DrawData, EmbedOptions};

/// At most this many swapchains are remembered; the least recently used goes first. A process
/// has one or two; the bound only matters for one that makes and drops many.
const MAX_STATES: usize = 8;

pub(super) enum Host {
    D3D11 {
        device: ID3D11Device,
        host: Option<Host11>,
    },
    D3D12 {
        device: ID3D12Device,
        queue: ID3D12CommandQueue,
        host: Option<Host12>,
    },
}

pub(super) struct SwapchainState {
    pub host: Host,
    pub luid: i64,
    /// The device (or queue) the swapchain was made with, to notice an address reused by a
    /// new swapchain.
    pub device_address: usize,
    pub target: Option<SharedTarget>,
    pub subclassed: bool,
    pub last_use: u64,
}

// SAFETY: the Direct3D objects are free-threaded, and a state is only used under the table's lock.
unsafe impl Send for SwapchainState {}

pub(super) enum Entry {
    Ready(Box<SwapchainState>),
    /// This swapchain cannot be drawn on, and why; not tried again.
    Unsupported,
}

pub(super) struct Table {
    pub entries: HashMap<usize, Entry>,
    pub clock: u64,
}

pub(super) static TABLE: Mutex<Option<Table>> = Mutex::new(None);

impl SwapchainState {
    pub fn new(swapchain: &IDXGISwapChain) -> Result<Self, String> {
        // SAFETY: plain queries on a live swapchain.
        unsafe {
            if let Ok(device) = swapchain.GetDevice::<ID3D11Device>() {
                let dxgi: IDXGIDevice = device.cast().map_err(|e| e.to_string())?;
                let adapter: IDXGIAdapter = dxgi.GetAdapter().map_err(|e| e.to_string())?;
                let luid = adapter.GetDesc().map_err(|e| e.to_string())?.AdapterLuid;
                return Ok(Self::of(
                    Host::D3D11 {
                        device: device.clone(),
                        host: None,
                    },
                    luid.HighPart,
                    luid.LowPart,
                    device.as_raw() as usize,
                ));
            }
            if let Ok(queue) = swapchain.GetDevice::<ID3D12CommandQueue>() {
                let mut found: Option<ID3D12Device> = None;
                queue.GetDevice(&mut found).map_err(|e| e.to_string())?;
                let device = found.ok_or("the queue has no device")?;
                let luid = device.GetAdapterLuid();
                let address = queue.as_raw() as usize;
                return Ok(Self::of(
                    Host::D3D12 {
                        device,
                        queue,
                        host: None,
                    },
                    luid.HighPart,
                    luid.LowPart,
                    address,
                ));
            }
        }
        Err("the swapchain belongs to neither Direct3D 11 nor Direct3D 12".into())
    }

    fn of(host: Host, high: i32, low: u32, device_address: usize) -> Self {
        Self {
            host,
            luid: wgpu_side::luid_key(high, low),
            device_address,
            target: None,
            subclassed: false,
            last_use: 0,
        }
    }

    /// The device or queue of the swapchain now, for comparison with `device_address`.
    pub fn device_address_of(swapchain: &IDXGISwapChain) -> usize {
        // SAFETY: plain queries on a live swapchain.
        unsafe {
            if let Ok(device) = swapchain.GetDevice::<ID3D11Device>() {
                return device.as_raw() as usize;
            }
            swapchain
                .GetDevice::<ID3D12CommandQueue>()
                .map_or(0, |queue| queue.as_raw() as usize)
        }
    }
}

/// Run `f` on the swapchain's entry, making it first. `None` when another thread has the table.
pub(super) fn with_entry<R>(
    swapchain: &IDXGISwapChain,
    f: impl FnOnce(&mut Entry) -> R,
) -> Option<R> {
    let mut guard = TABLE.try_lock().ok()?;
    let table = guard.get_or_insert_with(|| Table {
        entries: HashMap::new(),
        clock: 0,
    });
    table.clock += 1;
    let key = swapchain.as_raw() as usize;
    let address = SwapchainState::device_address_of(swapchain);
    let stale = matches!(table.entries.get(&key), Some(Entry::Ready(state)) if state.device_address != address);
    if stale {
        table.entries.remove(&key);
    }
    if !table.entries.contains_key(&key) {
        if table.entries.len() >= MAX_STATES {
            let oldest = table
                .entries
                .iter()
                .min_by_key(|(_, entry)| match entry {
                    Entry::Ready(state) => state.last_use,
                    Entry::Unsupported => 0,
                })
                .map(|(key, _)| *key);
            if let Some(oldest) = oldest {
                table.entries.remove(&oldest);
            }
        }
        let entry = match SwapchainState::new(swapchain) {
            Ok(state) => Entry::Ready(Box::new(state)),
            Err(reason) => {
                log::info!("z-hook: not drawing on swapchain {key:#x}: {reason}");
                Entry::Unsupported
            }
        };
        table.entries.insert(key, entry);
    }
    let clock = table.clock;
    let entry = table.entries.get_mut(&key)?;
    if let Entry::Ready(state) = entry {
        state.last_use = clock;
    }
    Some(f(entry))
}

/// Forget the shared texture of a swapchain about to be resized.
pub(super) fn drop_target(swapchain: &IDXGISwapChain) {
    if let Ok(mut guard) = TABLE.lock() {
        if let Some(Entry::Ready(state)) = guard
            .as_mut()
            .and_then(|t| t.entries.get_mut(&(swapchain.as_raw() as usize)))
        {
            state.target = None;
            match &mut state.host {
                Host::D3D11 { host, .. } => *host = None,
                Host::D3D12 { host, .. } => *host = None,
            }
        }
    }
}

pub(super) fn clear() {
    if let Ok(mut guard) = TABLE.lock() {
        *guard = None;
    }
}

/// Draws on one swapchain's backbuffer for one present.
pub(super) struct DxgiBackend<'a> {
    pub state: &'a mut SwapchainState,
    pub swapchain: &'a IDXGISwapChain,
    pub output: OutputColor,
    plan: Option<FormatPlan>,
    side: Option<Arc<WgpuSide>>,
}

impl<'a> DxgiBackend<'a> {
    pub fn new(
        state: &'a mut SwapchainState,
        swapchain: &'a IDXGISwapChain,
        output: OutputColor,
    ) -> Self {
        Self {
            state,
            swapchain,
            output,
            plan: None,
            side: None,
        }
    }

    fn ensure_target(
        &mut self,
        side: &WgpuSide,
        size: [u32; 2],
        plan: FormatPlan,
    ) -> Result<(), BackendError> {
        if self
            .state
            .target
            .as_ref()
            .is_some_and(|t| t.size == size && t.plan == plan)
        {
            return Ok(());
        }
        let mut target = SharedTarget::create(side, size, plan).map_err(BackendError::Frame)?;
        match &mut self.state.host {
            Host::D3D11 { device, host } => {
                *host = Some(Host11::open(device, &mut target).map_err(BackendError::Unsupported)?)
            }
            Host::D3D12 {
                device,
                queue,
                host,
            } => {
                *host = Some(
                    Host12::open(device, queue, &mut target).map_err(BackendError::Unsupported)?,
                )
            }
        }
        self.state.target = Some(target);
        Ok(())
    }
}

impl PresentBackend for DxgiBackend<'_> {
    fn begin(&mut self) -> Result<SurfaceInfo, BackendError> {
        // SAFETY: plain query on a live swapchain.
        let desc =
            unsafe { self.swapchain.GetDesc() }.map_err(|e| BackendError::Lost(e.to_string()))?;
        if desc.SampleDesc.Count > 1 {
            return Err(BackendError::Unsupported("the backbuffer is multisampled (bit-block swapchain); the overlay draws after the host's resolve only".into()));
        }
        let plan = plan(desc.BufferDesc.Format.0 as u32, self.output)
            .map_err(BackendError::Unsupported)?;
        let side = wgpu_side::get(self.state.luid, EmbedOptions::new(plan.texture))?;
        let size = [desc.BufferDesc.Width, desc.BufferDesc.Height];
        if size[0] == 0 || size[1] == 0 {
            return Err(BackendError::Frame("empty backbuffer".into()));
        }
        self.ensure_target(&side, size, plan)?;
        if let Host::D3D12 {
            host: Some(host), ..
        } = &self.state.host
        {
            if !host.ready() {
                return Err(BackendError::Frame(
                    "the previous overlay frame is still on the GPU".into(),
                ));
            }
        }
        if !self.state.subclassed {
            self.state.subclassed = crate::win32::attach(desc.OutputWindow);
        }
        self.plan = Some(plan);
        self.side = Some(side);
        let dpi = dpi_of(desc.OutputWindow);
        Ok(SurfaceInfo {
            size,
            scale_factor: dpi,
        })
    }

    fn render(&mut self, data: &DrawData) -> Result<(), BackendError> {
        if data.indices.is_empty() {
            return Ok(());
        }
        let side = self.side.clone().ok_or(BackendError::NotReady)?;
        let (Some(target), plan) = (self.state.target.as_mut(), self.plan) else {
            return Err(BackendError::NotReady);
        };
        let _ = plan;
        // The fence only counts up, whatever becomes of this frame.
        let base = target.value;
        target.value = base + 2;
        // SAFETY: the buffers are fetched and dropped within this call, so none is held across
        // a `ResizeBuffers`.
        unsafe {
            match &mut self.state.host {
                Host::D3D11 {
                    host: Some(host), ..
                } => {
                    let buffer: ID3D11Texture2D = self
                        .swapchain
                        .GetBuffer(0)
                        .map_err(|e| BackendError::Lost(e.to_string()))?;
                    let resource: ID3D11Resource = buffer
                        .cast()
                        .map_err(|e| BackendError::Lost(e.to_string()))?;
                    host.copy_in(&resource, base + 1)
                        .map_err(BackendError::Frame)?;
                    side.render(target, data, base + 1, base + 2)?;
                    host.copy_out(&resource, base + 2)
                        .map_err(BackendError::Frame)
                }
                Host::D3D12 {
                    host: Some(host), ..
                } => {
                    let index = self
                        .swapchain
                        .cast::<IDXGISwapChain3>()
                        .map_or(0, |sc| sc.GetCurrentBackBufferIndex());
                    let buffer: ID3D12Resource = self
                        .swapchain
                        .GetBuffer(index)
                        .map_err(|e| BackendError::Lost(e.to_string()))?;
                    host.copy_in(&buffer, base + 1)
                        .map_err(BackendError::Frame)?;
                    if let Err(error) = side.render(target, data, base + 1, base + 2) {
                        let _ = host.abandon();
                        return Err(error);
                    }
                    host.copy_out(&buffer, base + 2)
                        .map_err(BackendError::Frame)
                }
                _ => Err(BackendError::NotReady),
            }
        }
    }

    fn end(&mut self) -> Result<(), BackendError> {
        self.side = None;
        Ok(())
    }
}

fn dpi_of(hwnd: HWND) -> Option<f64> {
    // SAFETY: a plain query; zero means the window is gone.
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    (dpi != 0).then(|| f64::from(dpi) / 96.0)
}
