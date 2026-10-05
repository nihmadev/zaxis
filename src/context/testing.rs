//! Simulated input and read-only state inspection for integration tests.
//!
//! [`Driver`] injects input the way the winit event path does; [`Probe`] borrows
//! retained internal state so tests can assert on it. Neither is a stable API.

#![allow(private_interfaces)]

use super::*;
use crate::{Rect, Vec2};
use std::sync::Arc;
use winit::{event::ElementState, keyboard::KeyCode};

/// Input and focus injection for [`Context`].
pub trait Driver {
    fn move_pointer(&mut self, pointer: Vec2);
    fn primary_button(&mut self, state: ElementState) -> bool;
    fn secondary_button(&mut self, state: ElementState) -> bool;
    fn middle_button(&mut self, state: ElementState) -> bool;
    fn key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool;
    fn scroll_wheel(&mut self, delta: Vec2) -> bool;
    fn scroll_from(&mut self, id: Id, delta: Vec2, middle: bool) -> bool;
    fn set_focus(&mut self, focus: Option<Id>);
    fn set_modifiers(&mut self, modifiers: winit::keyboard::ModifiersState);
    fn dismiss_popup(&mut self, restore_focus: bool);
    fn clipboard_text(&mut self) -> Result<String, ClipboardError>;
    fn copy_text(&mut self, text: String) -> Result<(), ClipboardError>;
    fn take_slider_input(&mut self, id: Id) -> Vec<SliderInput>;
    fn text_carets(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
    ) -> Vec<(usize, f32)>;
    fn measure_text(&mut self, text: &str, size: f32, font: impl Into<TextFont>, wrap: f32)
        -> Vec2;
    fn paragraph_layout(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
        tab: u16,
    ) -> Arc<crate::text::TextLayout>;
    fn report(
        &mut self,
        kind: DiagnosticKind,
        id: Option<Id>,
        rect: Option<Rect>,
        message: impl FnOnce() -> String,
    );
    fn paint(&mut self, id: Id, layer: Id, clip: Rect, paint: Vec<Paint>);
    fn register_hit(&mut self, hit: HitRegion);
}

impl Driver for Context {
    fn move_pointer(&mut self, pointer: Vec2) {
        Context::move_pointer(self, pointer)
    }
    fn primary_button(&mut self, state: ElementState) -> bool {
        Context::primary_button(self, state)
    }
    fn secondary_button(&mut self, state: ElementState) -> bool {
        Context::secondary_button(self, state)
    }
    fn middle_button(&mut self, state: ElementState) -> bool {
        Context::middle_button(self, state)
    }
    fn key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool {
        Context::key(self, code, state, repeat)
    }
    fn scroll_wheel(&mut self, delta: Vec2) -> bool {
        Context::scroll_wheel(self, delta)
    }
    fn scroll_from(&mut self, id: Id, delta: Vec2, middle: bool) -> bool {
        Context::scroll_from(self, id, delta, middle)
    }
    fn set_focus(&mut self, focus: Option<Id>) {
        Context::set_focus(self, focus)
    }
    fn set_modifiers(&mut self, modifiers: winit::keyboard::ModifiersState) {
        self.input.modifiers = modifiers;
    }
    fn dismiss_popup(&mut self, restore_focus: bool) {
        Context::dismiss_popup(self, restore_focus)
    }
    fn clipboard_text(&mut self) -> Result<String, ClipboardError> {
        Context::clipboard_text(self)
    }
    fn copy_text(&mut self, text: String) -> Result<(), ClipboardError> {
        Context::copy_text(self, text)
    }
    fn take_slider_input(&mut self, id: Id) -> Vec<SliderInput> {
        Context::take_slider_input(self, id)
    }
    fn text_carets(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
    ) -> Vec<(usize, f32)> {
        Context::text_carets(self, text, size, font)
    }
    fn measure_text(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
    ) -> Vec2 {
        Context::measure_text(self, text, size, font, wrap)
    }
    fn paragraph_layout(
        &mut self,
        text: &str,
        size: f32,
        font: impl Into<TextFont>,
        wrap: f32,
        tab: u16,
    ) -> Arc<crate::text::TextLayout> {
        Context::paragraph_layout(self, text, size, font, wrap, tab)
    }
    fn report(
        &mut self,
        kind: DiagnosticKind,
        id: Option<Id>,
        rect: Option<Rect>,
        message: impl FnOnce() -> String,
    ) {
        Context::report(self, kind, id, rect, message)
    }
    fn paint(&mut self, id: Id, layer: Id, clip: Rect, paint: Vec<Paint>) {
        Context::paint(self, id, layer, clip, paint)
    }
    fn register_hit(&mut self, hit: HitRegion) {
        Context::register_hit(self, hit)
    }
}

/// Read-only view of the retained state behind a [`Context`].
pub trait Inspect {
    fn probe(&self) -> Probe<'_>;
    fn focus_visible(&self, id: Id) -> bool;
    fn probe_mut(&mut self) -> ProbeMut<'_>;
    fn hovered(&self, id: Id, window: Id, rect: Rect, clip: Rect) -> bool;
    fn top_modal_id(&self) -> Option<Id>;
    fn input_blocked(&self) -> bool;
    fn hit_test(&self, pointer: Vec2) -> Option<HitRegion>;
    fn clicked(&self, id: Id) -> bool;
    fn top_window(&self, pointer: Vec2) -> Option<Id>;
    fn front_window(&self) -> Option<Id>;
    fn layer_rank(&self, id: Id) -> usize;
}

/// Mutable view of the retained state behind a [`Context`].
pub struct ProbeMut<'a> {
    pub cache: &'a mut HashMap<Id, CachedElement>,
    pub scrolling: &'a mut scroll::Scrolling,
    pub drag: &'a mut drag::DragRuntime,
    pub placements: &'a mut placement::Placements,
    pub visual_depth: &'a mut usize,
    pub slider_input: &'a mut HashMap<Id, Vec<SliderInput>>,
    pub next_repaint: &'a mut Option<Instant>,
    pub text: &'a mut TextSystem,
    pub elements: &'a mut Vec<Element>,
    pub logical_size: &'a mut Vec2,
    pub modified: &'a mut HashSet<Id>,
    pub seen: &'a mut HashSet<Id>,
    pub stats: &'a mut CacheStats,
    pub style: &'a mut Style,
    pub native_chrome: &'a mut Option<native_chrome::NativeChrome>,
    pub draw_data: &'a mut DrawData,
}

pub struct Probe<'a> {
    pub input: &'a InputState,
    pub style: &'a Style,
    pub scale: f32,
    pub dirty: bool,
    pub next_repaint: Option<Instant>,
    pub frame: u64,
    pub frame_time: Instant,
    pub style_revision: u64,
    pub local_style_serial: u64,
    pub visual_depth: usize,
    pub stats: CacheStats,
    pub cache: &'a HashMap<Id, CachedElement>,
    pub visual_meshes: &'a HashMap<Id, paint::VisualMesh>,
    pub input_transforms: &'a HashMap<Id, crate::Transform>,
    pub effect_states: &'a HashMap<Id, crate::components::effects::EffectState>,
    pub elements: &'a [Element],
    /// Hit regions registered so far in the current pass.
    pub hits: &'a [HitRegion],
    pub previous_hits: &'a [HitRegion],
    pub capture: &'a Option<interaction::Capture>,
    pub clicked: &'a HashSet<Id>,
    pub slider_input: &'a HashMap<Id, Vec<SliderInput>>,
    pub combo_boxes: &'a HashMap<Id, crate::components::combo_box::ComboBoxState>,
    pub popup: &'a Option<popup::PopupState>,
    pub popup_layers: &'a [Id],
    pub modals: &'a modal::Modals,
    pub color_pickers: &'a HashMap<Id, crate::components::color_picker::ColorPickerState>,
    pub text_edits: &'a HashMap<Id, crate::components::text_edit::TextEditState>,
    pub ime_area: Option<Rect>,
    pub focused_widget: Option<Id>,
    pub windows: &'a HashMap<Id, WindowState>,
    pub tab_pages: &'a HashMap<Id, crate::components::motion::TabPagesState>,
    pub visible_windows: &'a HashSet<Id>,
    pub scrolling: &'a scroll::Scrolling,
    pub placements: &'a placement::Placements,
    pub cards: &'a HashMap<Id, crate::components::card::CardState>,
    pub carousels: &'a HashMap<Id, crate::components::carousel::CarouselState>,
    pub carousel_wheel_queued: usize,
    pub grids: &'a HashMap<Id, crate::components::grid::GridState>,
    pub list_boxes: &'a HashMap<Id, crate::components::list_box::ListState>,
    pub drag: &'a drag::DragRuntime,
    pub counts: Counts,
    pub seen: &'a HashSet<Id>,
    pub native_chrome: &'a Option<native_chrome::NativeChrome>,
    pub draw_data: &'a DrawData,
}

/// Sizes of retained per-widget tables, for cleanup assertions.
pub struct Counts {
    pub context_menus: usize,
    pub splits: usize,
    pub collapsing_headers: usize,
    pub split_input: usize,
    pub tree_input: usize,
    pub selection_items: usize,
    pub selection_scopes: usize,
}

impl Inspect for Context {
    fn probe(&self) -> Probe<'_> {
        Probe {
            input: &self.input,
            style: &self.style,
            scale: self.scale,
            dirty: self.dirty,
            next_repaint: self.next_repaint,
            frame: self.frame,
            frame_time: self.frame_time,
            style_revision: self.style_revision,
            local_style_serial: self.local_style_serial,
            visual_depth: self.visuals.depth,
            stats: self.stats,
            cache: &self.paint_state.cache,
            visual_meshes: &self.paint_state.visual_meshes,
            input_transforms: self.visuals.published(),
            effect_states: &self.visuals.effects,
            elements: &self.paint_state.elements,
            hits: &self.interaction.hits,
            previous_hits: &self.interaction.previous_hits,
            capture: &self.interaction.capture,
            clicked: &self.interaction.clicked,
            slider_input: &self.values.sliders_input,
            combo_boxes: &self.menus.combo_boxes,
            popup: &self.popups.current,
            popup_layers: &self.popups.layers,
            modals: &self.modals,
            color_pickers: &self.values.color_pickers,
            text_edits: &self.text_fields.states,
            ime_area: self.ime_area,
            focused_widget: self.interaction.focused,
            windows: &self.windows,
            tab_pages: &self.containers.tab_pages,
            visible_windows: &self.visible_windows,
            scrolling: &self.scrolling,
            placements: &self.placements,
            cards: &self.containers.cards,
            carousels: &self.containers.carousels,
            carousel_wheel_queued: self.carousel_wheel.queued(),
            grids: &self.containers.grids,
            list_boxes: &self.containers.list_boxes,
            drag: &self.drag,
            seen: &self.paint_state.seen,
            native_chrome: &self.native_chrome,
            draw_data: &self.geometry.draw_data,
            counts: Counts {
                context_menus: self.menus.context_menus.len(),
                splits: self.splits.states.len(),
                collapsing_headers: self.containers.collapsing_headers.len(),
                split_input: self.splits.pending(),
                tree_input: self.trees.pending(),
                selection_items: self.selection.items.len(),
                selection_scopes: self.selection.scopes.len(),
            },
        }
    }
    fn focus_visible(&self, id: Id) -> bool {
        Context::focus_visible(self, id)
    }
    fn probe_mut(&mut self) -> ProbeMut<'_> {
        ProbeMut {
            cache: &mut self.paint_state.cache,
            scrolling: &mut self.scrolling,
            drag: &mut self.drag,
            placements: &mut self.placements,
            visual_depth: &mut self.visuals.depth,
            slider_input: &mut self.values.sliders_input,
            next_repaint: &mut self.next_repaint,
            text: &mut self.text,
            elements: &mut self.paint_state.elements,
            logical_size: &mut self.logical_size,
            modified: &mut self.paint_state.modified,
            seen: &mut self.paint_state.seen,
            stats: &mut self.stats,
            style: &mut self.style,
            native_chrome: &mut self.native_chrome,
            draw_data: &mut self.geometry.draw_data,
        }
    }
    fn hovered(&self, id: Id, window: Id, rect: Rect, clip: Rect) -> bool {
        Context::hovered(self, id, window, rect, clip)
    }
    fn top_modal_id(&self) -> Option<Id> {
        Context::top_modal_id(self)
    }
    fn input_blocked(&self) -> bool {
        Context::input_blocked(self)
    }
    fn hit_test(&self, pointer: Vec2) -> Option<HitRegion> {
        Context::hit_test(self, pointer)
    }
    fn clicked(&self, id: Id) -> bool {
        Context::clicked(self, id)
    }
    fn top_window(&self, pointer: Vec2) -> Option<Id> {
        Context::top_window(self, pointer)
    }
    fn front_window(&self) -> Option<Id> {
        Context::front_window(self)
    }
    fn layer_rank(&self, id: Id) -> usize {
        Context::layer_rank(self, id)
    }
}

pub use super::clipboard::MemoryClipboard;
pub use super::drag::{preview::layer_id as drag_preview_layer, state::Payload};
pub use super::geometry::Element;
pub use super::interaction::{HitAction, HitRegion};
pub use super::paint::{CachedElement, Paint};
pub use super::values::SliderInput;
#[cfg(feature = "accesskit")]
pub use crate::accessibility::{testing::AccessTree, AccessStats};
pub use crate::components::list_box::{heights::HeightIndex, state::ListState};
pub use crate::components::scroll_area::RowMetrics;
pub use crate::text::{TextFont, DEFAULT_TAB};
