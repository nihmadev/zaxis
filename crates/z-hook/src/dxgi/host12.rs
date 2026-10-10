//! A Direct3D 12 host: copies recorded on command lists of our own and executed on the host's
//! queue, ordered with the overlay by the shared fence.
//!
//! The backbuffer is in the present state when the host calls `Present`; the copies move it
//! to a copy state and back. The shared texture needs no barrier: it has simultaneous access,
//! is promoted to a copy state and decays back to common.

use super::shared::SharedTarget;
use std::mem::ManuallyDrop;
use windows::Win32::Graphics::Direct3D12::*;

/// Frames of command-list memory in flight at once.
const SLOTS: usize = 3;

pub(super) struct Host12 {
    queue: ID3D12CommandQueue,
    texture: ID3D12Resource,
    fence: ID3D12Fence,
    allocators: Vec<ID3D12CommandAllocator>,
    copy_in: ID3D12GraphicsCommandList,
    copy_out: ID3D12GraphicsCommandList,
    /// Signaled on the host's queue when a frame's lists are done, so allocators can be reused.
    done: ID3D12Fence,
    done_value: u64,
    pending: [u64; SLOTS],
    slot: usize,
}

// SAFETY: Direct3D 12 objects are free-threaded; the lists are only recorded by the thread
// that presents, one frame at a time.
unsafe impl Send for Host12 {}

fn transition(
    resource: &ID3D12Resource,
    before: D3D12_RESOURCE_STATES,
    after: D3D12_RESOURCE_STATES,
) -> D3D12_RESOURCE_BARRIER {
    D3D12_RESOURCE_BARRIER {
        Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION,
        Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE,
        Anonymous: D3D12_RESOURCE_BARRIER_0 {
            Transition: ManuallyDrop::new(D3D12_RESOURCE_TRANSITION_BARRIER {
                // SAFETY: a borrowed copy of the pointer: `ManuallyDrop` never releases it.
                pResource: unsafe { std::mem::transmute_copy(resource) },
                Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES,
                StateBefore: before,
                StateAfter: after,
            }),
        },
    }
}

impl Host12 {
    pub fn open(
        device: &ID3D12Device,
        queue: &ID3D12CommandQueue,
        target: &mut SharedTarget,
    ) -> Result<Self, String> {
        let (resource, fence) = target.handles();
        // SAFETY: the handles were made by the overlay's device for exactly these objects.
        let (texture, fence): (ID3D12Resource, ID3D12Fence) = unsafe {
            let (mut texture, mut shared) = (None, None);
            device
                .OpenSharedHandle(resource, &mut texture)
                .map_err(|e| format!("OpenSharedHandle: {e}"))?;
            device
                .OpenSharedHandle(fence, &mut shared)
                .map_err(|e| format!("OpenSharedHandle: {e}"))?;
            (texture.ok_or("no texture")?, shared.ok_or("no fence")?)
        };
        target.close_handles();
        // SAFETY: plain object creation on a live device.
        unsafe {
            let allocators = (0..SLOTS)
                .map(|_| {
                    device.CreateCommandAllocator::<ID3D12CommandAllocator>(
                        D3D12_COMMAND_LIST_TYPE_DIRECT,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| format!("CreateCommandAllocator: {e}"))?;
            let list = |index| -> Result<ID3D12GraphicsCommandList, String> {
                let list: ID3D12GraphicsCommandList = device
                    .CreateCommandList(0, D3D12_COMMAND_LIST_TYPE_DIRECT, &allocators[index], None)
                    .map_err(|e| format!("CreateCommandList: {e}"))?;
                list.Close().map_err(|e| e.to_string())?;
                Ok(list)
            };
            let done = device
                .CreateFence(0, D3D12_FENCE_FLAG_NONE)
                .map_err(|e| format!("CreateFence: {e}"))?;
            Ok(Self {
                queue: queue.clone(),
                texture,
                fence,
                copy_in: list(0)?,
                copy_out: list(1)?,
                allocators,
                done,
                done_value: 0,
                pending: [0; SLOTS],
                slot: 0,
            })
        }
    }

    /// Whether the command memory of the next frame is free: the host's GPU may still be on it.
    pub fn ready(&self) -> bool {
        // SAFETY: a plain query.
        unsafe { self.done.GetCompletedValue() >= self.pending[self.slot] }
    }

    /// Copy the backbuffer into the shared texture and signal `value` on the shared fence.
    pub fn copy_in(&mut self, backbuffer: &ID3D12Resource, value: u64) -> Result<(), String> {
        let allocator = &self.allocators[self.slot];
        // SAFETY: the allocator's previous lists are done (checked by `ready`).
        unsafe {
            allocator
                .Reset()
                .map_err(|e| format!("allocator reset: {e}"))?;
            self.copy_in
                .Reset(allocator, None)
                .map_err(|e| e.to_string())?;
            self.copy_in.ResourceBarrier(&[transition(
                backbuffer,
                D3D12_RESOURCE_STATE_PRESENT,
                D3D12_RESOURCE_STATE_COPY_SOURCE,
            )]);
            self.copy_in.CopyResource(&self.texture, backbuffer);
            self.copy_in.ResourceBarrier(&[transition(
                backbuffer,
                D3D12_RESOURCE_STATE_COPY_SOURCE,
                D3D12_RESOURCE_STATE_PRESENT,
            )]);
            self.copy_in.Close().map_err(|e| e.to_string())?;
            self.queue
                .ExecuteCommandLists(&[Some(self.copy_in.clone().into())]);
            self.queue
                .Signal(&self.fence, value)
                .map_err(|e| format!("Signal: {e}"))
        }
    }

    /// The frame failed after `copy_in`: nothing more is recorded, and the allocator is released
    /// for reuse once the host's GPU has passed the copy.
    pub fn abandon(&mut self) -> Result<(), String> {
        self.done_value += 1;
        // SAFETY: a plain signal on the host's queue.
        unsafe { self.queue.Signal(&self.done, self.done_value) }
            .map_err(|e| format!("Signal: {e}"))?;
        self.pending[self.slot] = self.done_value;
        self.slot = (self.slot + 1) % SLOTS;
        Ok(())
    }

    /// Wait for the overlay (`value`), copy the result back, and mark the frame's lists done.
    pub fn copy_out(&mut self, backbuffer: &ID3D12Resource, value: u64) -> Result<(), String> {
        let allocator = &self.allocators[self.slot];
        // SAFETY: as in `copy_in`; `copy_in` is closed and executed, so the allocator may record
        // the next list.
        unsafe {
            self.copy_out
                .Reset(allocator, None)
                .map_err(|e| e.to_string())?;
            self.copy_out.ResourceBarrier(&[transition(
                backbuffer,
                D3D12_RESOURCE_STATE_PRESENT,
                D3D12_RESOURCE_STATE_COPY_DEST,
            )]);
            self.copy_out.CopyResource(backbuffer, &self.texture);
            self.copy_out.ResourceBarrier(&[transition(
                backbuffer,
                D3D12_RESOURCE_STATE_COPY_DEST,
                D3D12_RESOURCE_STATE_PRESENT,
            )]);
            self.copy_out.Close().map_err(|e| e.to_string())?;
            self.queue
                .Wait(&self.fence, value)
                .map_err(|e| format!("Wait: {e}"))?;
            self.queue
                .ExecuteCommandLists(&[Some(self.copy_out.clone().into())]);
            self.done_value += 1;
            self.queue
                .Signal(&self.done, self.done_value)
                .map_err(|e| format!("Signal: {e}"))?;
        }
        self.pending[self.slot] = self.done_value;
        self.slot = (self.slot + 1) % SLOTS;
        Ok(())
    }
}
