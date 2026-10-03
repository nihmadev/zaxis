//! Context construction and unique draw-data sources.

use super::{CacheStats, Context, InputState};
use crate::{text::TextSystem, DrawData, Style, Vec2};
use ab_glyph::FontArc;
use std::collections::{HashMap, HashSet};

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Construct a context with the bundled OFL-licensed Lato font and grey style.
    pub fn new() -> Self {
        Self::with_font(
            FontArc::try_from_slice(include_bytes!("../../assets/Lato-Regular.ttf"))
                .expect("bundled font is valid"),
        )
    }

    /// Replace the default font without relying on platform font discovery.
    pub fn with_font(font: FontArc) -> Self {
        let draw_data = DrawData::default();
        let ids = crate::images::TextureIds::default();
        Self {
            native_chrome: None,
            input: InputState {
                focused: true,
                ..Default::default()
            },
            style: Style::default(),
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
            frame_time: std::time::Instant::now(),
            in_pass: false,
            animations: Default::default(),
            text: TextSystem::with_allocator(font, ids.clone()),
            images: crate::images::ImageCache::new(draw_data.source, ids),
            image_visual_scale: 1.0,
            cache: HashMap::new(),
            visual_meshes: HashMap::new(),
            visual_depth: 0,
            visual_clips: Vec::new(),
            visual_materializing: false,
            input_transforms: HashMap::new(),
            current_transforms: HashMap::new(),
            effect_states: HashMap::new(),
            elements: Vec::new(),
            paint_order: HashMap::new(),
            previous_elements: Vec::new(),
            mesh_slots: Vec::new(),
            seen: HashSet::new(),
            modified: HashSet::new(),
            hits: Vec::new(),
            hit_order: HashMap::new(),
            previous_hits: Vec::new(),
            capture: None,
            text_click: None,
            clicked: HashSet::new(),
            slider_input: HashMap::new(),
            text_edit_input: HashMap::new(),
            number_input: HashMap::new(),
            numbers: HashMap::new(),
            combo_boxes: HashMap::new(),
            combo_input: HashMap::new(),
            context_menus: HashMap::new(),
            secondary_target: None,
            popup: None,
            dismissed_popups: HashSet::new(),
            popup_layers: Vec::new(),
            tooltips: Default::default(),
            color_pickers: HashMap::new(),
            text_edits: HashMap::new(),
            clipboard: None,
            ime_composing: false,
            ime_area: None,
            ime_target: None,
            focused_widget: None,
            focus_visible: false,
            keyboard_active: None,
            windows: HashMap::new(),
            tab_pages: HashMap::new(),
            layers: Vec::new(),
            visible_windows: HashSet::new(),
            draw_data,
            stats: CacheStats::default(),
            scrolling: Default::default(),
            auto_ids: HashMap::new(),
            placements: Default::default(),
            grids: HashMap::new(),
            layouts: HashMap::new(),
            tables: HashMap::new(),
            column_resize: HashMap::new(),
            splits: HashMap::new(),
            split_input: HashMap::new(),
            split_click: None,
            collapsing_headers: HashMap::new(),
            trees: HashMap::new(),
            tree_input: HashMap::new(),
            tree_click: None,
        }
    }
}
