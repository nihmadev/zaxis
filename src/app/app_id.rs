//! The application identifier windows carry: Wayland `app_id`, X11 `WM_CLASS`.

use winit::window::WindowAttributes;

/// Used when the executable name is unknown.
const FALLBACK: &str = "zaxis";

/// The identifier of a window: its own, else the run-wide one, else the executable name.
pub fn resolve(window: Option<&str>, run: Option<&str>) -> String {
    [window, run]
        .into_iter()
        .flatten()
        .find(|id| !id.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(executable_name)
}

fn executable_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| FALLBACK.to_owned())
}

/// Set the Wayland `app_id` and the X11 class and instance. Other platforms ignore it.
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "android")))]
pub fn apply(attributes: WindowAttributes, id: &str) -> WindowAttributes {
    use winit::platform::{wayland::WindowAttributesExtWayland, x11::WindowAttributesExtX11};
    let attributes = WindowAttributesExtWayland::with_name(attributes, id, id);
    WindowAttributesExtX11::with_name(attributes, id, id)
}

#[cfg(not(all(unix, not(target_os = "macos"), not(target_os = "android"))))]
pub fn apply(attributes: WindowAttributes, _id: &str) -> WindowAttributes {
    attributes
}
