//! The overlay's own Direct3D 12 device, made through wgpu on the host's adapter.

use crate::backend::BackendError;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, PoisonError,
    },
};
use windows::Win32::Graphics::Dxgi::IDXGIAdapter3;
use zaxis::{wgpu, EmbedLoad, EmbedOptions, EmbeddedRenderer};

pub(super) struct WgpuSide {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub renderer: Mutex<EmbeddedRenderer>,
    _adapter: wgpu::Adapter,
    _instance: wgpu::Instance,
}

enum Slot {
    Starting,
    Ready(Arc<WgpuSide>),
    Failed(String),
}

/// One slot per adapter (by LUID); created in the background.
static SIDES: Mutex<Option<HashMap<i64, Slot>>> = Mutex::new(None);
static CLOSING: AtomicBool = AtomicBool::new(false);

/// The side for the adapter `luid`, once it is made. The first call starts making it.
pub(super) fn get(luid: i64, options: EmbedOptions) -> Result<Arc<WgpuSide>, BackendError> {
    let mut sides = SIDES.lock().unwrap_or_else(PoisonError::into_inner);
    let sides = sides.get_or_insert_with(HashMap::new);
    match sides.get(&luid) {
        Some(Slot::Ready(side)) => return Ok(Arc::clone(side)),
        Some(Slot::Starting) => return Err(BackendError::NotReady),
        Some(Slot::Failed(reason)) => return Err(BackendError::Unsupported(reason.clone())),
        None => {}
    }
    sides.insert(luid, Slot::Starting);
    let spawned = std::thread::Builder::new()
        .name("z-hook-init".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(|| create(luid, options));
            let slot = match result {
                Ok(Ok(side)) => Slot::Ready(Arc::new(side)),
                Ok(Err(reason)) => Slot::Failed(reason),
                Err(payload) => {
                    Slot::Failed(format!("panic: {}", crate::driver::panic_message(&payload)))
                }
            };
            if !CLOSING.load(Ordering::Acquire) {
                if let Some(sides) = SIDES
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .as_mut()
                {
                    sides.insert(luid, slot);
                }
            }
        });
    if let Err(error) = spawned {
        return Err(BackendError::Unsupported(error.to_string()));
    }
    Err(BackendError::NotReady)
}

/// Drop every side (the overlay is being removed).
pub(super) fn release() {
    CLOSING.store(true, Ordering::Release);
    *SIDES.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

pub(super) fn reopen() {
    CLOSING.store(false, Ordering::Release);
}

fn create(luid: i64, options: EmbedOptions) -> Result<WgpuSide, String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12));
    let adapter = adapters
        .into_iter()
        .find(|adapter| adapter_luid(adapter) == Some(luid))
        .ok_or("wgpu does not see the host's adapter")?;
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("z-hook"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .map_err(|error| error.to_string())?;
    let renderer =
        EmbeddedRenderer::new_with_adapter(device.clone(), queue.clone(), &adapter, options)
            .map_err(|error| error.to_string())?;
    Ok(WgpuSide {
        device,
        queue,
        renderer: Mutex::new(renderer),
        _adapter: adapter,
        _instance: instance,
    })
}

fn adapter_luid(adapter: &wgpu::Adapter) -> Option<i64> {
    // SAFETY: the adapter is a Direct3D 12 one when `as_hal` returns it.
    let hal = unsafe { adapter.as_hal::<wgpu::hal::api::Dx12>() }?;
    let raw: &IDXGIAdapter3 = hal.raw_adapter();
    // SAFETY: a plain query on a live adapter.
    let desc = unsafe { raw.GetDesc() }.ok()?;
    Some(luid_key(
        desc.AdapterLuid.HighPart,
        desc.AdapterLuid.LowPart,
    ))
}

pub(super) fn luid_key(high: i32, low: u32) -> i64 {
    (i64::from(high) << 32) | i64::from(low)
}

impl WgpuSide {
    /// Draw `data` into the shared target: wait for the host's copy at `wait`, signal `signal`.
    pub fn render(
        &self,
        target: &super::shared::SharedTarget,
        data: &zaxis::DrawData,
        wait: u64,
        signal: u64,
    ) -> Result<(), BackendError> {
        use wgpu::hal::api::Dx12;
        let plan = target.plan;
        let extent = wgpu::Extent3d {
            width: target.size[0],
            height: target.size[1],
            depth_or_array_layers: 1,
        };
        let view_formats = [plan.view];
        let view_formats: &[wgpu::TextureFormat] = if plan.view != plan.texture {
            &view_formats
        } else {
            &[]
        };
        // SAFETY: the resource was created on this device with these properties; the host's
        // other API shares it through the fence, so `PRESENT` (common) is the state it is in.
        let texture = unsafe {
            let hal = wgpu::hal::dx12::Device::texture_from_raw(
                target.resource.clone(),
                plan.texture,
                wgpu::TextureDimension::D2,
                extent,
                1,
                1,
            );
            self.device.create_texture_from_hal::<Dx12>(
                hal,
                &wgpu::TextureDescriptor {
                    label: Some("host backbuffer copy"),
                    size: extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: plan.texture,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats,
                },
                wgpu::TextureUses::PRESENT,
            )
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(plan.view),
            ..Default::default()
        });
        let mut renderer = self.renderer.lock().unwrap_or_else(PoisonError::into_inner);
        let options = EmbedOptions::new(plan.texture);
        if *renderer.options() != options {
            renderer
                .set_options(options)
                .map_err(|e| BackendError::Unsupported(e.to_string()))?;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("z-hook"),
            });
        renderer
            .render_to(&mut encoder, &view, None, EmbedLoad::Keep, data)
            .map_err(|error| BackendError::Frame(error.to_string()))?;
        drop(renderer);
        // Back to the common state, which is what the host's API expects of a shared texture.
        let mut last = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("z-hook share"),
            });
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("z-hook share state"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        }));
        // SAFETY: a fresh Direct3D 12 encoder, not touched while the closure records.
        unsafe {
            last.as_hal_mut::<Dx12, _, _>(|raw| {
                use wgpu::hal::CommandEncoder as _;
                let (Some(raw), Some(hal_texture)) = (raw, texture.as_hal::<Dx12>()) else {
                    return;
                };
                raw.transition_textures(std::iter::once(wgpu::hal::TextureBarrier {
                    texture: &*hal_texture,
                    range: wgpu::ImageSubresourceRange::default(),
                    usage: wgpu::hal::StateTransition {
                        from: wgpu::TextureUses::COLOR_TARGET,
                        to: wgpu::TextureUses::PRESENT,
                    },
                }));
            });
        }
        let commands = [encoder.finish(), last.finish()];
        // SAFETY: the queue is a Direct3D 12 one; the fence is shared with the host.
        let hal_queue = unsafe { self.queue.as_hal::<Dx12>() }
            .ok_or_else(|| BackendError::Unsupported("not a Direct3D 12 queue".into()))?;
        hal_queue.add_wait_fence(target.fence.clone(), wait);
        hal_queue.add_signal_fence(target.fence.clone(), signal);
        drop(hal_queue);
        self.queue.submit(commands);
        let _ = self.device.poll(wgpu::PollType::Poll);
        Ok(())
    }
}
