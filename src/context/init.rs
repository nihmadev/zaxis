//! Context construction and unique draw-data sources.

use super::{CacheStats, Context, InputState, SharedResources};
use crate::{text::TextSystem, FontFamily, Vec2};
use ab_glyph::{Font, FontArc};
use std::collections::{HashMap, HashSet};

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Construct a context with the bundled OFL-licensed Inter family and the dark theme.
    pub fn new() -> Self {
        Self::with_fonts(FontFamily::inter())
    }

    /// Replace the default font with a single regular face, without relying on
    /// platform font discovery. Use [`Context::with_fonts`] for several weights.
    pub fn with_font(font: FontArc) -> Self {
        Self::with_fonts(FontFamily::new(font.font_data().to_vec()))
    }

    /// Replace the default font with a family of per-weight font files. System
    /// fonts still supply scripts the family lacks, followed by color emoji.
    /// The context gets resources of its own; use [`Context::with_shared`] for windows
    /// that should share them.
    pub fn with_fonts(family: FontFamily) -> Self {
        Self::build(SharedResources::with_fonts(family))
    }

    /// Like [`Context::with_fonts`] with an explicit monospace family, which stays
    /// independent of the main one. `None` selects the system's generic monospace font.
    pub fn with_font_families(family: FontFamily, monospace: Option<FontFamily>) -> Self {
        Self::build(SharedResources::with_font_families(family, monospace))
    }

    pub(super) fn build(shared: SharedResources) -> Self {
        let file_notify = crate::files::Notify::new(shared.images().clone());
        Self {
            native_chrome: None,
            input: InputState {
                focused: true,
                ..Default::default()
            },
            style: crate::Theme::dark().resolve(),
            theme: None,
            style_revision: 0,
            local_styles: HashMap::new(),
            local_style_serial: 0,
            palette_transition: None,
            logical_size: Vec2::ZERO,
            scale: 1.0,
            dirty: true,
            next_repaint: None,
            frame: 0,
            frame_time: crate::time::Instant::now(),
            in_pass: false,
            stats: CacheStats::default(),
            auto_ids: HashMap::new(),
            diagnostics: Default::default(),
            field_status: Default::default(),
            animations: Default::default(),
            text: TextSystem::with_store(
                shared.font_family().clone(),
                shared.monospace_family().cloned(),
                shared.glyphs(),
            ),
            images: shared.images().clone(),
            shared,
            appearance_seen: 0,
            images_epoch: 0,
            shows_images: false,
            clipboard: None,
            paint_state: Default::default(),
            geometry: Default::default(),
            visuals: Default::default(),
            placements: Default::default(),
            scrolling: Default::default(),
            materials: Default::default(),
            interaction: Default::default(),
            gestures: Default::default(),
            windows: HashMap::new(),
            layers: Vec::new(),
            visible_windows: HashSet::new(),
            popups: Default::default(),
            modals: Default::default(),
            tooltips: Default::default(),
            toasts: Default::default(),
            drag: Default::default(),
            files: Default::default(),
            #[cfg(feature = "file-dialogs")]
            dialogs: super::dialogs::Dialogs::new(file_notify.clone()),
            file_notify,
            ime_area: None,
            ime_target: None,
            text_fields: Default::default(),
            values: Default::default(),
            menus: Default::default(),
            tables: Default::default(),
            trees: Default::default(),
            splits: Default::default(),
            containers: Default::default(),
            carousel_wheel: Default::default(),
            selection: Default::default(),
            key_capture: Default::default(),
            a11y: Default::default(),
        }
    }
}
