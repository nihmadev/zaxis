//! What to hook and how the overlay behaves.

use std::time::Duration;
use zaxis::{
    winit::keyboard::{KeyCode, ModifiersState},
    FontFamily,
};

/// A graphics API whose presentation can be hooked.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Api {
    /// Through an implicit layer (`vkQueuePresentKHR`). Linux and Windows.
    Vulkan,
    /// `IDXGISwapChain::Present` of a Direct3D 12 swapchain. Windows only.
    D3D12,
    /// `IDXGISwapChain::Present` of a Direct3D 11 swapchain. Windows only.
    D3D11,
    /// `wglSwapBuffers`, `glXSwapBuffers`, `eglSwapBuffers`. Not implemented; see the
    /// documentation for why.
    OpenGl,
}

impl Api {
    pub const ALL: [Api; 4] = [Api::Vulkan, Api::D3D12, Api::D3D11, Api::OpenGl];

    fn bit(self) -> u8 {
        1 << self as u8
    }

    /// Whether this API can be hooked on the platform this crate was built for.
    pub const fn available(self) -> bool {
        match self {
            Api::Vulkan => cfg!(all(feature = "vulkan", any(unix, windows))),
            Api::D3D12 | Api::D3D11 => cfg!(windows),
            Api::OpenGl => false,
        }
    }
}

/// A set of [`Api`]s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ApiSet(u8);

impl ApiSet {
    pub const fn none() -> Self {
        Self(0)
    }

    /// Every API that [`Api::available`] on this platform.
    pub fn available() -> Self {
        Api::ALL
            .into_iter()
            .filter(|api| api.available())
            .fold(Self::none(), Self::with)
    }

    pub fn with(mut self, api: Api) -> Self {
        self.0 |= api.bit();
        self
    }

    pub fn without(mut self, api: Api) -> Self {
        self.0 &= !api.bit();
        self
    }

    pub fn contains(self, api: Api) -> bool {
        self.0 & api.bit() != 0
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = Api> {
        Api::ALL.into_iter().filter(move |api| self.contains(*api))
    }
}

impl FromIterator<Api> for ApiSet {
    fn from_iter<T: IntoIterator<Item = Api>>(iter: T) -> Self {
        iter.into_iter().fold(Self::none(), Self::with)
    }
}

/// Who receives keyboard and mouse input while the overlay is visible.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum OverlayInput {
    /// The host receives all input, always. The interface still sees a copy, so it can show
    /// state and react to the toggle key, but it never takes anything away from the host.
    PassThrough,
    /// While visible, the interface takes all keyboard and mouse input; the host gets none.
    CaptureWhenVisible,
    /// While visible, the interface takes what one of its controls uses: pointer events over
    /// the interface and keys while a control has keyboard focus. The rest goes to the host.
    #[default]
    CaptureWhenFocused,
}

/// A key (with modifiers) that shows and hides the overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToggleKey {
    pub key: KeyCode,
    pub modifiers: ModifiersState,
}

impl ToggleKey {
    pub const fn new(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: ModifiersState::empty(),
        }
    }

    pub const fn with_modifiers(mut self, modifiers: ModifiersState) -> Self {
        self.modifiers = modifiers;
        self
    }
}

/// How colors are encoded in the host's backbuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum OutputColor {
    /// 8 and 10 bit backbuffers hold sRGB values (also when their format is `UNORM`); float
    /// backbuffers hold linear values.
    #[default]
    Auto,
    /// Encode as sRGB. Not available for float backbuffers.
    Srgb,
    /// Write linear values. Only float backbuffers can hold them.
    Linear,
}

/// Overlay configuration. Build with the methods; the defaults suit a debug overlay toggled
/// with F10 that takes input only where its controls use it.
#[derive(Clone)]
pub struct OverlayOptions {
    pub apis: ApiSet,
    pub toggle: Option<ToggleKey>,
    pub input: OverlayInput,
    pub fonts: Option<FontFamily>,
    pub monospace: Option<FontFamily>,
    pub output: OutputColor,
    /// DPI scale factor of the host's window. `None`: ask the platform where it can
    /// (a window's DPI on Windows), otherwise 1.0.
    pub scale_factor: Option<f64>,
    pub start_visible: bool,
    /// What one overlay frame (interface pass plus recording) may cost on the host's render
    /// thread. A frame that exceeds it makes the next ones pass through, in proportion.
    /// `None`: no limit.
    pub frame_budget: Option<Duration>,
}

impl Default for OverlayOptions {
    fn default() -> Self {
        Self {
            apis: ApiSet::available(),
            toggle: Some(ToggleKey::new(KeyCode::F10)),
            input: OverlayInput::default(),
            fonts: None,
            monospace: None,
            output: OutputColor::Auto,
            scale_factor: None,
            start_visible: true,
            frame_budget: Some(Duration::from_millis(6)),
        }
    }
}

impl OverlayOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apis(mut self, apis: ApiSet) -> Self {
        self.apis = apis;
        self
    }

    pub fn toggle(mut self, toggle: Option<ToggleKey>) -> Self {
        self.toggle = toggle;
        self
    }

    pub fn input(mut self, input: OverlayInput) -> Self {
        self.input = input;
        self
    }

    /// The family of the interface, as for [`Context::with_fonts`](zaxis::Context::with_fonts).
    pub fn fonts(mut self, fonts: FontFamily) -> Self {
        self.fonts = Some(fonts);
        self
    }

    pub fn monospace(mut self, monospace: FontFamily) -> Self {
        self.monospace = Some(monospace);
        self
    }

    pub fn output(mut self, output: OutputColor) -> Self {
        self.output = output;
        self
    }

    pub fn scale_factor(mut self, scale_factor: f64) -> Self {
        self.scale_factor = Some(scale_factor);
        self
    }

    pub fn start_visible(mut self, visible: bool) -> Self {
        self.start_visible = visible;
        self
    }

    pub fn frame_budget(mut self, budget: Option<Duration>) -> Self {
        self.frame_budget = budget;
        self
    }
}
