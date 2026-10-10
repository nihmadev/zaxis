//! A Direct3D 11 host: copies on its immediate context, ordered by the shared fence.
//!
//! `CopyResource`, `Signal` and `Wait` are not pipeline state, so nothing of the host's has
//! to be saved or restored around them.

use super::shared::SharedTarget;
use windows::{core::Interface, Win32::Graphics::Direct3D11::*};

pub(super) struct Host11 {
    context: ID3D11DeviceContext,
    context4: ID3D11DeviceContext4,
    texture: ID3D11Texture2D,
    fence: ID3D11Fence,
}

impl Host11 {
    /// Open the shared target on the host's device.
    pub fn open(device: &ID3D11Device, target: &mut SharedTarget) -> Result<Self, String> {
        let device1: ID3D11Device1 = device
            .cast()
            .map_err(|_| "the device is older than Direct3D 11.1")?;
        let device5: ID3D11Device5 = device
            .cast()
            .map_err(|_| "the device has no shared fences (Windows 10 1703)")?;
        let (resource, fence) = target.handles();
        // SAFETY: the handles were made by the overlay's device for exactly these objects.
        let texture: ID3D11Texture2D = unsafe { device1.OpenSharedResource1(resource) }
            .map_err(|e| format!("OpenSharedResource1: {e}"))?;
        let mut opened: Option<ID3D11Fence> = None;
        // SAFETY: as above.
        unsafe { device5.OpenSharedFence(fence, &mut opened) }
            .map_err(|e| format!("OpenSharedFence: {e}"))?;
        let fence = opened.ok_or("no fence")?;
        target.close_handles();
        // SAFETY: plain query.
        let context: ID3D11DeviceContext =
            unsafe { device.GetImmediateContext() }.map_err(|e| e.to_string())?;
        let context4: ID3D11DeviceContext4 = context
            .cast()
            .map_err(|_| "the context has no fence operations")?;
        Ok(Self {
            context,
            context4,
            texture,
            fence,
        })
    }

    /// Copy the backbuffer into the shared texture and signal `value`.
    pub fn copy_in(&self, backbuffer: &ID3D11Resource, value: u64) -> Result<(), String> {
        // SAFETY: both resources are live and share a format family and size.
        unsafe {
            self.context.CopyResource(&self.texture, backbuffer);
            self.context4
                .Signal(&self.fence, value)
                .map_err(|e| format!("Signal: {e}"))
        }
    }

    /// Wait for the overlay (`value`), then copy the result back into the backbuffer.
    pub fn copy_out(&self, backbuffer: &ID3D11Resource, value: u64) -> Result<(), String> {
        // SAFETY: as in `copy_in`.
        unsafe {
            self.context4
                .Wait(&self.fence, value)
                .map_err(|e| format!("Wait: {e}"))?;
            self.context.CopyResource(backbuffer, &self.texture);
        }
        Ok(())
    }
}
