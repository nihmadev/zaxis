//! A library for Windows hosts that draws a zaxis window over their Direct3D 11 or 12
//! swapchains.
//!
//! The host (a game's mod loader, a tool of yours, `d3d11_host --overlay <path>`) loads the DLL
//! and calls `zaxis_overlay_start` from a thread of its own: the hooks are placed there, never
//! in `DllMain`, which the loader lock forbids. `zaxis_overlay_stop` removes them again.

#[cfg(windows)]
mod windows_only {
    use std::sync::Mutex;
    use z_hook::{Overlay, OverlayOptions};
    use zaxis::{Context, Slider, Window};

    static OVERLAY: Mutex<Option<Overlay>> = Mutex::new(None);

    /// Install the overlay. Returns 0 on success, 1 if it is installed already, 2 on failure.
    #[no_mangle]
    pub extern "C" fn zaxis_overlay_start() -> i32 {
        z_hook::log::init_from_env();
        let mut slot = OVERLAY.lock().unwrap_or_else(|p| p.into_inner());
        if slot.is_some() {
            return 1;
        }
        let mut level = 0.5_f32;
        let mut enabled = true;
        let ui = move |ctx: &mut Context| {
            Window::new("Overlay").show(ctx, |ui| {
                ui.add(Slider::new(&mut level, 0.0..=1.0));
                ui.checkbox(&mut enabled, "Enabled");
            });
        };
        match Overlay::install(OverlayOptions::default(), ui) {
            Ok(overlay) => {
                *slot = Some(overlay);
                0
            }
            Err(error) => {
                log::error!("z-hook: {error}");
                2
            }
        }
    }

    /// Remove the hooks. The library can be unloaded after this returns.
    #[no_mangle]
    pub extern "C" fn zaxis_overlay_stop() {
        OVERLAY.lock().unwrap_or_else(|p| p.into_inner()).take();
    }
}

#[cfg(not(windows))]
fn _only_on_windows() {}
