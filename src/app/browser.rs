//! Browser settings and the rules for what the page keeps. Plain data and pure functions,
//! so they exist on every target; only the `wasm32` runner reads them.

/// Which graphics API the browser runner uses.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WebBackend {
    /// WebGPU where the browser has it, otherwise WebGL2.
    #[default]
    Auto,
    /// WebGPU only; creating the renderer fails where the browser lacks it.
    WebGpu,
    /// WebGL2 only, even where WebGPU exists. For testing the fallback.
    WebGl2,
}

impl WebBackend {
    /// Parse the value of the `zaxis_backend` URL parameter: `webgpu`, `webgl` or `webgl2`.
    /// The wgpu backends to try for this choice, in order of preference.
    pub fn backends(self) -> wgpu::Backends {
        match self {
            Self::Auto => wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
            Self::WebGpu => wgpu::Backends::BROWSER_WEBGPU,
            Self::WebGl2 => wgpu::Backends::GL,
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "webgpu" => Some(Self::WebGpu),
            "webgl" | "webgl2" | "gl" => Some(Self::WebGl2),
            _ => None,
        }
    }

    /// The backend named by a URL query such as `?zaxis_backend=webgl2`.
    pub fn from_query(query: &str) -> Option<Self> {
        query
            .trim_start_matches('?')
            .split('&')
            .find_map(|pair| pair.strip_prefix("zaxis_backend="))
            .and_then(Self::from_name)
    }
}

/// Where and how the browser runner draws. Native runners ignore it.
///
/// Without `canvas_id` the runner creates a `<canvas>`. Appended to `<body>` it covers the
/// whole browser window; appended to `container_id` it fills that element, whose size the
/// page decides. A canvas found by `canvas_id` keeps the size the page's CSS gives it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebOptions {
    /// `id` of an existing `<canvas>` to draw on.
    pub canvas_id: Option<String>,
    /// `id` of the element that receives the canvas the runner creates.
    pub container_id: Option<String>,
    /// The graphics API. The `zaxis_backend` URL parameter overrides [`WebBackend::Auto`].
    pub backend: WebBackend,
}

/// Whether the browser's own reaction to a key press over the canvas must be cancelled.
///
/// The page keeps the shortcuts that belong to the browser (reload, find, developer tools,
/// zoom, tab switching) and the clipboard keys, whose `copy`/`cut`/`paste` events the runner
/// needs. It takes what the UI uses for itself: focus traversal, scrolling and cursor keys,
/// typing, select-all, and anything the UI reported as consumed.
pub fn blocks_browser_default(
    key: &str,
    ctrl: bool,
    meta: bool,
    alt: bool,
    consumed: bool,
) -> bool {
    let command = ctrl || meta;
    if command {
        let clipboard = matches!(key.to_ascii_lowercase().as_str(), "c" | "x" | "v");
        let named_and_consumed = consumed && key.chars().count() > 1;
        return !clipboard && (key.eq_ignore_ascii_case("a") || named_and_consumed);
    }
    if alt {
        return false;
    }
    let navigation = matches!(
        key,
        "Tab"
            | " "
            | "Backspace"
            | "Enter"
            | "ArrowUp"
            | "ArrowDown"
            | "ArrowLeft"
            | "ArrowRight"
            | "PageUp"
            | "PageDown"
            | "Home"
            | "End"
    );
    navigation || key.chars().count() == 1 || consumed && !key.starts_with('F')
}
