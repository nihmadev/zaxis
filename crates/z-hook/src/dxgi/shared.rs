//! The texture and fence shared between the host's device and the overlay's.

use super::wgpu_side::WgpuSide;
use crate::dxgi_format::FormatPlan;
use windows::Win32::{
    Foundation::{CloseHandle, HANDLE},
    Graphics::{
        Direct3D12::*,
        Dxgi::Common::{DXGI_FORMAT, DXGI_SAMPLE_DESC},
    },
};
use zaxis::wgpu::hal::api::Dx12;

/// `GENERIC_ALL`, the access a shared handle is made with.
const GENERIC_ALL: u32 = 0x1000_0000;

/// A texture the size of the host's backbuffer, and the fence ordering the two devices'
/// work on it. `value` counts the fence: each frame advances it by two (the host's copy-in
/// signals the first, the overlay's drawing the second).
pub(super) struct SharedTarget {
    pub resource: ID3D12Resource,
    pub fence: ID3D12Fence,
    resource_handle: HANDLE,
    fence_handle: HANDLE,
    pub size: [u32; 2],
    pub plan: FormatPlan,
    pub value: u64,
}

// SAFETY: Direct3D 12 objects are free-threaded; handles are plain values.
unsafe impl Send for SharedTarget {}

impl SharedTarget {
    pub fn create(side: &WgpuSide, size: [u32; 2], plan: FormatPlan) -> Result<Self, String> {
        // SAFETY: the device is a Direct3D 12 one.
        let hal = unsafe { side.device.as_hal::<Dx12>() }
            .ok_or("the overlay's device is not Direct3D 12")?;
        let device = hal.raw_device();
        let heap = D3D12_HEAP_PROPERTIES {
            Type: D3D12_HEAP_TYPE_DEFAULT,
            ..Default::default()
        };
        let description = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: u64::from(size[0]),
            Height: size[1],
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT(plan.typeless as i32),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET
                | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS,
            ..Default::default()
        };
        let mut resource: Option<ID3D12Resource> = None;
        // SAFETY: plain creation with valid descriptors.
        unsafe {
            device.CreateCommittedResource(
                &heap,
                D3D12_HEAP_FLAG_SHARED,
                &description,
                D3D12_RESOURCE_STATE_COMMON,
                None,
                &mut resource,
            )
        }
        .map_err(|e| format!("CreateCommittedResource: {e}"))?;
        let resource = resource.ok_or("no resource")?;
        // SAFETY: plain creation.
        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_SHARED) }
            .map_err(|e| format!("CreateFence: {e}"))?;
        // SAFETY: both objects were created shareable on this device.
        let (resource_handle, fence_handle) = unsafe {
            (
                device
                    .CreateSharedHandle(&resource, None, GENERIC_ALL, None)
                    .map_err(|e| format!("CreateSharedHandle: {e}"))?,
                device
                    .CreateSharedHandle(&fence, None, GENERIC_ALL, None)
                    .map_err(|e| format!("CreateSharedHandle: {e}"))?,
            )
        };
        Ok(Self {
            resource,
            fence,
            resource_handle,
            fence_handle,
            size,
            plan,
            value: 0,
        })
    }

    /// The handles the host opens its views of the target with.
    pub fn handles(&self) -> (HANDLE, HANDLE) {
        (self.resource_handle, self.fence_handle)
    }

    /// Close this side's handles once the host has opened what they name.
    pub fn close_handles(&mut self) {
        for handle in [&mut self.resource_handle, &mut self.fence_handle] {
            if !handle.is_invalid() {
                // SAFETY: a handle this value created and has not closed.
                let _ = unsafe { CloseHandle(*handle) };
                *handle = HANDLE::default();
            }
        }
    }
}

impl Drop for SharedTarget {
    fn drop(&mut self) {
        self.close_handles();
    }
}
