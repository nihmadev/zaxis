//! Browser settings and the key policy, which exist on every target so they can be checked
//! without a browser.
use zaxis::app::browser::blocks_browser_default as blocks;
use zaxis::{wgpu::Backends, WebBackend};

#[test]
fn the_page_keeps_browser_shortcuts_and_clipboard_keys() {
    // Reload, find, developer tools, zoom, tab and window management stay with the browser.
    assert!(!blocks("r", true, false, false, false));
    assert!(!blocks("f", false, true, false, false));
    assert!(!blocks("F5", false, false, false, false));
    assert!(!blocks("F12", false, false, false, true));
    assert!(
        !blocks("ArrowLeft", false, false, true, true),
        "history navigation"
    );
    // Copy, cut and paste must reach the browser so its clipboard events fire.
    for key in ["c", "x", "v", "C", "V"] {
        assert!(!blocks(key, true, false, false, true), "{key}");
        assert!(!blocks(key, false, true, false, true), "{key}");
    }
}

#[test]
fn the_ui_keeps_its_own_keys() {
    for key in [
        "Tab",
        " ",
        "ArrowDown",
        "PageDown",
        "Home",
        "Backspace",
        "Enter",
    ] {
        assert!(blocks(key, false, false, false, false), "{key}");
    }
    // Typing, including the characters the browser would use for quick find.
    for key in ["a", "/", "'", "я"] {
        assert!(blocks(key, false, false, false, false), "{key}");
    }
    assert!(blocks("a", true, false, false, false), "select all");
    assert!(
        blocks("Delete", true, false, false, true),
        "consumed word deletion"
    );
    assert!(
        blocks("Escape", false, false, false, true),
        "consumed by the UI"
    );
    assert!(
        !blocks("Escape", false, false, false, false),
        "not consumed"
    );
}

#[test]
fn the_backend_comes_from_options_or_the_url() {
    assert_eq!(
        WebBackend::from_query("?zaxis_backend=webgl2"),
        Some(WebBackend::WebGl2)
    );
    assert_eq!(
        WebBackend::from_query("?a=1&zaxis_backend=WebGPU"),
        Some(WebBackend::WebGpu)
    );
    assert_eq!(WebBackend::from_query("?zaxis_backend=nonsense"), None);
    assert_eq!(WebBackend::from_query(""), None);
    assert_eq!(WebBackend::WebGl2.backends(), Backends::GL);
    assert_eq!(WebBackend::WebGpu.backends(), Backends::BROWSER_WEBGPU);
    assert!(WebBackend::Auto
        .backends()
        .contains(Backends::GL | Backends::BROWSER_WEBGPU));
}
