//! Handing a checked URL to whatever opens it: the system handler on the desktop, a new
//! browser tab in a page.

#[cfg(target_os = "windows")]
pub(super) fn launch(url: &str) -> std::io::Result<()> {
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(drop)
}

#[cfg(target_os = "macos")]
pub(super) fn launch(url: &str) -> std::io::Result<()> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map(drop)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_arch = "wasm32")))]
pub(super) fn launch(url: &str) -> std::io::Result<()> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map(drop)
}

/// A new tab without access to this page. Browsers allow it only shortly after a user
/// gesture, which the click that activated the link provides.
#[cfg(target_arch = "wasm32")]
pub(super) fn launch(url: &str) -> std::io::Result<()> {
    use std::io::{Error, ErrorKind};
    let window =
        web_sys::window().ok_or_else(|| Error::new(ErrorKind::Unsupported, "no window"))?;
    match window.open_with_url_and_target_and_features(url, "_blank", "noopener,noreferrer") {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(Error::new(
            ErrorKind::PermissionDenied,
            "the browser blocked the new tab",
        )),
        Err(_) => Err(Error::other("opening the new tab failed")),
    }
}
