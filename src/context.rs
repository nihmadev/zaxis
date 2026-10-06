//! Global UI state, winit input integration, repaint scheduling, and geometry cache.

mod animation;
mod carousel;
pub(crate) mod clipboard;
mod containers;
mod cursor;
mod debug_overlay;
mod diagnostics;
#[cfg(feature = "file-dialogs")]
mod dialogs;
pub mod drag;
#[doc(hidden)]
pub mod events;
pub(crate) mod file_drop;
mod file_io;
mod frame;
#[doc(hidden)]
pub mod geometry;
mod gesture;
mod id;
mod images;
mod ime;
mod init;
mod input;
mod interaction;
mod keyboard;
mod materials;
mod menus;
pub(crate) mod modal;
#[doc(hidden)]
pub mod native_chrome;
mod paint;
pub(crate) mod placement;
mod pointer;
pub(crate) mod popup;
mod repaint;
pub(crate) mod scroll;
mod scroll_input;
pub(crate) mod selection;
mod shared;
mod split;
mod table;
#[doc(hidden)]
pub mod testing;
mod text_api;
mod text_input;
mod theme;
pub(crate) mod toast;
#[doc(hidden)]
pub mod tooltip;
mod tree;
mod values;
mod viewport;
mod visual;
mod windows;

use crate::time::Instant;
use crate::{text::TextSystem, DrawData, Style, Vec2};
use std::collections::{HashMap, HashSet};

#[cfg(target_arch = "wasm32")]
pub(crate) use clipboard::web::install as install_web_clipboard;
pub use clipboard::{ClipboardBackend, ClipboardError};
pub(crate) use diagnostics::{invalid_value, Diagnostics};
pub use diagnostics::{DebugOverlay, Diagnostic, DiagnosticKind};
pub use id::Id;
pub use input::{EventResponse, InputState};
pub(crate) use interaction::{HitAction, HitRegion};
pub use materials::MaterialUse;
pub(crate) use paint::{MaterialImage, Paint};
pub use shared::SharedResources;
pub(crate) use text_input::TextEditInput;
pub(crate) use values::{NumberInputEvent, SliderInput};
pub(crate) use windows::WindowState;

/// Cache counters for profiling and tests. Each `run` increments UI passes;
/// unchanged paint descriptions reuse tessellation and frame geometry.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub ui_passes: u64,
    pub tessellated_elements: u64,
    pub reused_elements: u64,
    pub geometry_rebuilds: u64,
    pub geometry_full_rebuilds: u64,
    pub geometry_partial_updates: u64,
    pub geometry_bytes_copied: u64,
    /// Text layouts shaped from scratch; cache reuse does not count.
    pub text_layouts_built: u64,
}

/// Stateful IMGUI context. One context is intended for one native viewport: with several
/// windows, create one per window with [`Context::with_shared`] so that they share fonts,
/// the glyph atlas, the image cache and the theme but never input or retained widget state.
///
/// The context coordinates subsystems that each own their state and its lifecycle; see
/// [`Context::run_at`] for the order in which a pass drives them.
pub struct Context {
    // Viewport, clock, style and repaint scheduling.
    pub(crate) native_chrome: Option<native_chrome::NativeChrome>,
    input: InputState,
    style: Style,
    pub(crate) theme: Option<crate::Theme>,
    pub(crate) style_revision: u64,
    pub(crate) local_styles: HashMap<Id, (Style, u64, u64)>,
    pub(crate) local_style_serial: u64,
    pub(crate) palette_transition: Option<()>,
    logical_size: Vec2,
    scale: f32,
    dirty: bool,
    next_repaint: Option<Instant>,
    pub(crate) frame: u64,
    frame_time: Instant,
    in_pass: bool,
    stats: CacheStats,
    pub(crate) auto_ids: HashMap<Id, u64>,
    pub(crate) diagnostics: Diagnostics,
    /// Status of the enclosing `Field`, inherited by field-like controls.
    pub(crate) field_status: crate::SemanticStatus,

    // Resources: shared with other windows' contexts, or this context's own.
    pub(crate) animations: crate::animation::state::Animations,
    text: TextSystem,
    pub(crate) images: crate::images::SharedImages,
    shared: SharedResources,
    appearance_seen: u64,
    images_epoch: u64,
    shows_images: bool,
    clipboard: Option<Box<dyn ClipboardBackend>>,

    // Output of a pass: elements and cached meshes, then the frame buffers.
    /// Elements of this pass and meshes cached across passes.
    pub(crate) paint_state: paint::PaintState,
    /// Frame buffers built from the elements.
    geometry: geometry::FrameGeometry,
    /// Visual transforms, effects being built and retained effect state.
    pub(crate) visuals: visual::Visuals,
    pub(crate) placements: placement::Placements,
    pub(crate) scrolling: scroll::Scrolling,
    /// The clock of animated materials.
    pub(crate) materials: materials::MaterialState,

    // Who receives input: regions, capture, focus, gestures, layers and overlays.
    /// Hit regions, capture, focus and activations.
    pub(crate) interaction: interaction::Interaction,
    pub(crate) gestures: gesture::Gestures,
    pub(crate) windows: HashMap<Id, WindowState>,
    layers: Vec<Id>,
    visible_windows: HashSet<Id>,
    pub(crate) popups: popup::Popups,
    pub(crate) modals: modal::Modals,
    pub(crate) tooltips: tooltip::Tooltips,
    pub(crate) toasts: toast::Toasts,
    pub(crate) drag: drag::DragRuntime,
    /// Files dragged in from the system, and the notice background file work raises.
    pub(crate) files: file_drop::FileDrop,
    pub(crate) file_notify: crate::files::Notify,
    #[cfg(feature = "file-dialogs")]
    pub(crate) dialogs: dialogs::Dialogs,
    pub(crate) ime_area: Option<crate::Rect>,
    ime_target: Option<Id>,

    // Input routed to built-in controls and their retained state, by family.
    /// Text fields: routed keys, text, IME and presses, and field state.
    pub(crate) text_fields: text_input::TextFields,
    /// Sliders, drag values and numeric and color editors.
    pub(crate) values: values::ValueControls,
    /// Combo boxes, context menus and menu bars, and their navigation keys.
    pub(crate) menus: menus::Menus,
    pub(crate) tables: table::Tables,
    pub(crate) trees: tree::Trees,
    pub(crate) splits: split::Splits,
    pub(crate) containers: containers::Containers,
    pub(crate) carousel_wheel: carousel::CarouselWheel,
    /// Selection and retained state of static text; see [`selection`].
    pub(crate) selection: selection::SelectionState,
    pub(crate) key_capture: crate::components::key_box::KeyCapture,
    /// Nodes described for assistive technology and the tree built from them.
    pub(crate) a11y: crate::accessibility::State,
}

impl Context {
    /// Input as seen by the caller. While a modal is open, code outside it (global
    /// shortcuts, widgets under the overlay) reads idle input: no pointer, keys or text.
    pub fn input(&self) -> &InputState {
        if self.input_blocked() {
            self.blocked_input()
        } else {
            &self.input
        }
    }
    pub fn style(&self) -> &Style {
        &self.style
    }
    pub fn set_style(&mut self, style: Style) {
        self.theme = None;
        self.palette_transition = None;
        self.animations.remove(Id::new("theme-palette"));
        if self.style == style {
            return;
        }
        if style.motion.reduced_motion && !self.style.motion.reduced_motion {
            self.animations.reduce_motion(self.frame_time);
        }
        self.style = style;
        self.style_revision = self.style_revision.wrapping_add(1);
        self.request_repaint();
    }
    pub fn draw_data(&self) -> &DrawData {
        &self.geometry.draw_data
    }
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            text_layouts_built: self.text.builds,
            ..self.stats
        }
    }
}
