//! The one overlay of the process and the hooks armed for it.

use crate::{core::Core, error::HookError, options::Api};
use std::sync::{Arc, Mutex};

static CURRENT: Mutex<Option<Arc<Core>>> = Mutex::new(None);

fn current_slot() -> std::sync::MutexGuard<'static, Option<Arc<Core>>> {
    CURRENT
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub(crate) fn register(core: &Arc<Core>) -> Result<(), HookError> {
    let mut slot = current_slot();
    if slot.as_ref().is_some_and(|core| !core.is_disabled()) {
        return Err(HookError::AlreadyInstalled);
    }
    *slot = Some(Arc::clone(core));
    Ok(())
}

/// The overlay installed in this process, if there is one and it works.
#[allow(dead_code)] // used by the graphics API hooks
pub(crate) fn current() -> Option<Arc<Core>> {
    current_slot().clone().filter(|core| !core.is_disabled())
}

/// Place the hooks of the requested APIs. The Vulkan layer needs none: it is loaded by the
/// Vulkan loader and finds the overlay through [`current`].
pub(crate) fn arm(core: &Arc<Core>) -> Result<(), HookError> {
    let mut placed = 0;
    for api in core.options.apis.iter() {
        match api {
            Api::Vulkan if Api::Vulkan.available() => placed += 1,
            Api::OpenGl => log::warn!("z-hook: OpenGL is not supported; see the documentation"),
            #[cfg(windows)]
            Api::D3D11 | Api::D3D12 => {
                crate::dxgi::install(core, api)?;
                placed += 1;
            }
            _ => log::info!("z-hook: {api:?} is not available on this platform"),
        }
    }
    if placed == 0 {
        return Err(HookError::NoSupportedApi);
    }
    Ok(())
}

pub(crate) fn disarm(core: &Arc<Core>) {
    #[cfg(windows)]
    crate::dxgi::remove(core);
    let mut slot = current_slot();
    if slot
        .as_ref()
        .is_some_and(|current| Arc::ptr_eq(current, core))
    {
        *slot = None;
    }
}
