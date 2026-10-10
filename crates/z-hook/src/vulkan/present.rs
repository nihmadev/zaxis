//! `vkQueuePresentKHR`: where the overlay is drawn.

use super::{
    backend::VulkanBackend,
    ffi::guarded,
    state::{self, DeviceData, SwapchainData},
};
use ash::vk::{self, Handle};
use std::{slice, sync::Arc};

/// A swapchain smaller than this is a helper surface (a launcher, a splash), not the game.
const MIN_AREA: u64 = 128 * 128;

fn device_of(queue: vk::Queue) -> Option<Arc<DeviceData>> {
    state::with(|registry| {
        registry
            .queues
            .get(&queue.as_raw())
            .or_else(|| registry.devices.values().next())
            .cloned()
    })
}

/// The swapchain of this present the overlay draws on, with its image index: the chosen
/// target, or the first large one when none is chosen yet.
fn pick(
    swapchains: &[vk::SwapchainKHR],
    indices: &[u32],
) -> Option<(Arc<SwapchainData>, u32, bool)> {
    state::with(|registry| {
        let mut chosen = None;
        for (swapchain, index) in swapchains.iter().zip(indices) {
            let key = swapchain.as_raw();
            let Some(data) = registry.swapchains.get(&key) else {
                continue;
            };
            if registry.target == Some(key) {
                return Some((Arc::clone(data), *index, false));
            }
            if registry.target.is_none() && chosen.is_none() && data.area() >= MIN_AREA {
                chosen = Some((Arc::clone(data), *index, key));
            }
        }
        let (data, index, key) = chosen?;
        registry.target = Some(key);
        Some((data, index, true))
    })
}

pub(super) unsafe extern "system" fn queue_present(
    queue: vk::Queue,
    p_present_info: *const vk::PresentInfoKHR<'_>,
) -> vk::Result {
    // SAFETY: the arguments are the loader's and the host's, passed on unchanged.
    guarded(
        "queue_present",
        vk::Result::ERROR_INITIALIZATION_FAILED,
        || unsafe { queue_present_impl(queue, p_present_info) },
    )
}

unsafe fn queue_present_impl(
    queue: vk::Queue,
    p_present_info: *const vk::PresentInfoKHR<'_>,
) -> vk::Result {
    let Some(device) = device_of(queue) else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    };
    let present = device.swapchain.queue_present_khr;
    // SAFETY: the host's present info is valid for this call.
    let host = unsafe { &*p_present_info };
    let mut forwarded = *host;
    let mut signal = [vk::Semaphore::null()];
    guarded("present", (), || {
        let Some(core) = crate::registry::current() else {
            return;
        };
        if device.interop.is_none() || host.swapchain_count == 0 {
            return;
        }
        // SAFETY: Vulkan requires these arrays to hold `swapchain_count` / `wait_semaphore_count` entries.
        let (swapchains, indices, waits) = unsafe {
            (
                slice::from_raw_parts(host.p_swapchains, host.swapchain_count as usize),
                slice::from_raw_parts(host.p_image_indices, host.swapchain_count as usize),
                if host.wait_semaphore_count == 0 {
                    &[][..]
                } else {
                    slice::from_raw_parts(
                        host.p_wait_semaphores,
                        host.wait_semaphore_count as usize,
                    )
                },
            )
        };
        let Some((swapchain, image_index, chosen)) = pick(swapchains, indices) else {
            return;
        };
        if chosen {
            super::input::start(&core, &swapchain);
        }
        let Some(&position) = device
            .queues
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&queue.as_raw())
        else {
            return;
        };
        let mut backend = VulkanBackend::new(
            &swapchain,
            image_index,
            position,
            waits,
            core.options.output,
        );
        crate::driver::run_frame(&core, &mut backend);
        if let Some(semaphore) = backend.signal {
            signal[0] = semaphore;
            forwarded.wait_semaphore_count = 1;
            forwarded.p_wait_semaphores = signal.as_ptr();
        }
    });
    // SAFETY: the next layer's function with the (possibly rewritten) present info.
    unsafe { present(queue, &forwarded) }
}
