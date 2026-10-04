//! Global UI state, winit input integration, repaint scheduling, and geometry cache.

mod animation;
mod cursor;
mod debug_overlay;
mod diagnostics;
#[cfg(test)]
mod diagnostics_tests;
mod disclosure;
#[cfg(test)]
#[path = "../tests/disclosure/interaction.rs"]
mod disclosure_tests;
pub(crate) mod drag;
mod events;
mod frame;
mod geometry;
mod gesture;
mod id;
mod images;
mod ime;
mod init;
mod input;
mod interaction;
mod keyboard;
pub(crate) mod modal;
pub(crate) mod native_chrome;
mod paint;
pub(crate) mod placement;
mod pointer;
pub(crate) mod popup;
mod repaint;
pub(crate) mod scroll;
mod scroll_input;
#[cfg(test)]
mod scroll_input_tests;
mod split;
#[cfg(test)]
#[path = "../tests/split/button.rs"]
mod split_button_tests;
#[cfg(test)]
#[path = "../tests/split/events.rs"]
mod split_tests;
mod text_api;
mod theme;
#[cfg(test)]
mod theme_regression_tests;
#[cfg(test)]
mod theme_tests;
pub(crate) mod tooltip;
mod tree;
mod viewport;
mod windows;

#[cfg(test)]
mod animation_compose_tests;
#[cfg(test)]
mod animation_control_tests;
#[cfg(test)]
mod animation_motion_tests;
#[cfg(test)]
mod animation_tests;
#[cfg(test)]
mod api_ergonomics_tests;
#[cfg(test)]
mod card_tests;
#[cfg(test)]
mod color_picker_tests;
#[cfg(test)]
mod combo_box_tests;
#[cfg(test)]
#[path = "../tests/context_menu/interaction.rs"]
mod context_menu_tests;
#[cfg(test)]
mod cursor_tests;
#[cfg(test)]
mod drag_tests;
#[cfg(test)]
mod field_tests;
#[cfg(test)]
mod font_weight_tests;
#[cfg(test)]
mod grid_tests;
#[cfg(test)]
mod interact_tests;
#[cfg(test)]
mod layout_motion_tests;
#[cfg(test)]
mod modal_tests;
#[cfg(test)]
mod motion_presets_tests;
#[cfg(test)]
mod number_tests;
#[cfg(test)]
mod response_events_tests;
#[cfg(test)]
mod scroll_tests;
#[cfg(test)]
mod slider_tests;
#[cfg(test)]
mod switch_tests;
#[cfg(test)]
mod tab_bar_tests;
#[cfg(test)]
mod table_tests;
#[cfg(test)]
mod text_edit_tests;
#[cfg(test)]
mod text_area_tests;

use crate::{text::TextSystem, DrawData, Style, Vec2};
use geometry::{Element, ElementKey, MeshSlot};
use interaction::Capture;
use paint::CachedElement;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
use winit::keyboard::KeyCode;

pub(crate) use diagnostics::{invalid_value, Diagnostics};
pub use diagnostics::{DebugOverlay, Diagnostic, DiagnosticKind};
pub use id::Id;
pub use input::{EventResponse, InputState};
pub(crate) use interaction::{HitAction, HitRegion, NumberInputEvent, SliderInput, TextEditInput};
pub(crate) use paint::Paint;
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

/// Stateful IMGUI context. One context is intended for one native viewport.
pub struct Context {
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
    pub(crate) animations: crate::animation::state::Animations,
    text: TextSystem,
    pub(crate) images: crate::images::ImageCache,
    pub(crate) image_visual_scale: f32,
    cache: HashMap<Id, CachedElement>,
    pub(crate) visual_meshes: HashMap<Id, paint::VisualMesh>,
    pub(crate) visual_depth: usize,
    pub(crate) visual_clips: Vec<(Id, crate::Rect)>,
    pub(crate) visual_materializing: bool,
    pub(crate) input_transforms: HashMap<Id, crate::Transform>,
    pub(crate) current_transforms: HashMap<Id, crate::Transform>,
    pub(crate) effect_states: HashMap<Id, crate::components::effects::EffectState>,
    elements: Vec<Element>,
    paint_order: HashMap<Id, usize>,
    previous_elements: Vec<ElementKey>,
    mesh_slots: Vec<MeshSlot>,
    seen: HashSet<Id>,
    modified: HashSet<Id>,
    hits: Vec<HitRegion>,
    /// First registration of each hit ID: its order and, unless merely reserved, its window.
    hit_order: HashMap<Id, (usize, Option<Id>)>,
    previous_hits: Vec<HitRegion>,
    capture: Option<Capture>,
    text_click: Option<interaction::ClickSequence>,
    clicked: HashSet<Id>,
    slider_input: HashMap<Id, Vec<SliderInput>>,
    text_edit_input: HashMap<Id, Vec<TextEditInput>>,
    /// Text areas that take Tab as input (fields registered this pass / on the last pass).
    pub(crate) text_edit_tabs: HashSet<Id>,
    text_edit_tabs_previous: HashSet<Id>,
    number_input: HashMap<Id, Vec<NumberInputEvent>>,
    pub(crate) numbers: HashMap<Id, crate::components::number_input::NumberState>,
    pub(crate) combo_boxes: HashMap<Id, crate::components::combo_box::ComboBoxState>,
    pub(crate) combo_input: HashMap<Id, Vec<KeyCode>>,
    pub(crate) context_menus: HashMap<Id, crate::components::context_menu::MenuState>,
    pub(crate) gestures: gesture::Gestures,
    pub(crate) diagnostics: Diagnostics,
    /// Status of the enclosing `Field`, inherited by field-like controls.
    pub(crate) field_status: crate::SemanticStatus,
    pub(crate) popup: Option<popup::PopupState>,
    pub(crate) dismissed_popups: HashSet<Id>,
    pub(crate) popup_layers: Vec<Id>,
    pub(crate) modals: modal::Modals,
    pub(crate) tooltips: tooltip::Tooltips,
    pub(crate) color_pickers: HashMap<Id, crate::components::color_picker::ColorPickerState>,
    pub(crate) text_edits: HashMap<Id, crate::components::text_edit::TextEditState>,
    clipboard: Option<arboard::Clipboard>,
    ime_composing: bool,
    pub(crate) ime_area: Option<crate::Rect>,
    ime_target: Option<Id>,
    pub(crate) focused_widget: Option<Id>,
    focus_visible: bool,
    keyboard_active: Option<(Id, KeyCode)>,
    pub(crate) windows: HashMap<Id, WindowState>,
    pub(crate) tab_pages: HashMap<Id, crate::components::motion::TabPagesState>,
    layers: Vec<Id>,
    visible_windows: HashSet<Id>,
    draw_data: DrawData,
    stats: CacheStats,
    pub(crate) scrolling: scroll::Scrolling,
    pub(crate) auto_ids: HashMap<Id, u64>,
    pub(crate) placements: placement::Placements,
    pub(crate) grids: HashMap<Id, crate::components::grid::GridState>,
    pub(crate) cards: HashMap<Id, crate::components::card::CardState>,
    pub(crate) layouts: HashMap<Id, crate::components::ui::FlowState>,
    pub(crate) tables: HashMap<Id, crate::components::table::TableState>,
    column_resize: HashMap<(Id, Id), f32>,
    pub(crate) splits: HashMap<Id, crate::components::split_pane::SplitState>,
    split_input: HashMap<Id, Vec<crate::components::split_pane::SplitInput>>,
    split_click: Option<interaction::ClickSequence>,
    pub(crate) collapsing_headers:
        HashMap<Id, crate::components::collapsing_header::CollapsingState>,
    pub(crate) trees: HashMap<Id, crate::components::tree_view::TreeState>,
    pub(crate) tree_input: HashMap<Id, Vec<crate::components::tree_view::TreeInput>>,
    tree_click: Option<interaction::ClickSequence>,
    pub(crate) drag: drag::DragRuntime,
}

impl Context {
    /// Current visual bounds of a widget's logical allocation. During a pass,
    /// call after the enclosing visual/reorder helper has placed its content.
    pub fn visual_rect(&self, id: Id, logical: crate::Rect) -> crate::Rect {
        let transforms = if self.in_pass {
            &self.current_transforms
        } else {
            &self.input_transforms
        };
        transforms
            .get(&id)
            .copied()
            .unwrap_or_default()
            .rect(logical)
    }
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
        &self.draw_data
    }
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            text_layouts_built: self.text.builds,
            ..self.stats
        }
    }
}
