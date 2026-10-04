//! UI pass lifecycle and retained-state cleanup.

use super::Context;
use std::time::Instant;

impl Context {
    /// Build the UI on every host redraw, comparing paint descriptions to reuse geometry.
    /// Model changes are observed without explicit cache invalidation. Returns `true`.
    /// Clicks and value changes schedule a follow-up redraw through [`Self::needs_repaint`]
    /// so controls built before a mutation also observe the updated model.
    pub fn run(&mut self, build: impl FnOnce(&mut Self)) -> bool {
        self.run_at(Instant::now(), build)
    }

    /// Build with a host-provided monotonic time, shared by animations and timers.
    /// Backwards timestamps are clamped to the previous pass; long gaps are sampled
    /// directly, without simulating missed frames. Use `needs_repaint_at` with this
    /// same clock in deterministic/custom hosts.
    pub fn run_at(&mut self, now: Instant, build: impl FnOnce(&mut Self)) -> bool {
        self.frame_time = now.max(self.frame_time);
        self.in_pass = true;
        self.animations.begin_pass();
        if self
            .next_repaint
            .is_some_and(|time| time <= self.frame_time)
        {
            self.next_repaint = None;
        }
        // Clear before the callback so repaint requests made by widgets survive it.
        self.dirty = false;
        self.ime_area = None;
        self.frame += 1;
        self.sample_theme_palette();
        self.images.begin_frame(self.frame, self.frame_time);
        self.text.begin_frame();
        self.stats.ui_passes += 1;
        self.elements.clear();
        self.paint_order.clear();
        self.hits.clear();
        self.native_chrome = None;
        self.hit_order.clear();
        self.seen.clear();
        self.modified.clear();
        self.current_transforms.clear();
        self.auto_ids.clear();
        self.popup_layers.clear();
        self.modals.begin_pass();
        self.tick_auto_scroll();
        self.drag_begin_frame();
        self.scrolling.begin_frame();
        build(self);
        self.finish_frame();
        self.in_pass = false;
        true
    }

    pub(super) fn finish_frame(&mut self) {
        // Events were delivered to this pass; anything recorded from here on
        // (vanished widgets, cancelled captures) belongs to the next one.
        self.gestures.finish_frame();
        if self.popup.as_ref().is_some_and(|popup| {
            popup.last_frame != self.frame
                || !self
                    .windows
                    .get(&popup.owner)
                    .is_some_and(|w| w.last_frame == self.frame)
        }) {
            self.dismiss_popup(true);
        }
        self.visible_windows = self
            .windows
            .iter()
            .filter(|(_, window)| window.last_frame == self.frame)
            .map(|(id, _)| *id)
            .collect();
        self.finish_modals();
        self.finish_tooltips();
        self.drag_emit_preview();
        self.finish_diagnostics();
        let ranks: std::collections::HashMap<_, _> = self
            .layers
            .iter()
            .chain(self.popup_layers.iter())
            .copied()
            .map(|id| (id, self.layer_rank(id)))
            .collect();
        self.elements.sort_by_key(|e| {
            (
                ranks.get(&e.layer).copied().unwrap_or(0),
                self.paint_order.get(&e.id).copied().unwrap_or(0),
            )
        });
        self.hits.sort_by_key(|hit| {
            (
                ranks.get(&hit.window).copied().unwrap_or(0),
                self.hit_order.get(&hit.id).map_or(0, |slot| slot.0),
            )
        });
        self.hits.retain(|h| !h.rect.intersect(h.clip).is_empty());
        self.scrolling.finish_frame(self.frame);
        self.finish_auto_scroll();
        if let Some(deadline) = self.images.finish_frame() {
            self.request_repaint_after(deadline.saturating_duration_since(self.frame_time));
        }
        if std::mem::take(&mut self.images.state_changed) {
            self.request_repaint();
        }
        self.rebuild_geometry();
        self.text.end_frame();
        self.animations.finish_pass(self.frame);
        self.tab_pages
            .retain(|_, state| state.last_frame == self.frame);
        self.context_menus
            .retain(|_, state| state.last_frame == self.frame);
        self.effect_states
            .retain(|_, state| state.last_frame == self.frame);
        self.visual_meshes.retain(|id, _| self.seen.contains(id));
        self.current_transforms
            .retain(|id, _| self.seen.contains(id) || self.hits.iter().any(|hit| hit.id == *id));
        self.input_transforms = std::mem::take(&mut self.current_transforms);
        self.grids.retain(|_, state| state.last_frame == self.frame);
        self.cards.retain(|_, state| state.last_frame == self.frame);
        self.layouts
            .retain(|_, state| state.last_frame == self.frame);
        self.local_styles.retain(|_, s| s.2 == self.frame);
        self.previous_hits = std::mem::take(&mut self.hits);
        self.text_edit_tabs_previous = std::mem::take(&mut self.text_edit_tabs);
        self.drag_finish_state();
        self.cache
            .retain(|_, element| element.last_frame == self.frame);
        self.settle_modal_focus();
        if self.focused_widget.is_some_and(|id| {
            !self
                .previous_hits
                .iter()
                .any(|h| h.id == id && h.action.focusable())
        }) {
            self.set_focus(None);
            self.keyboard_active = None;
        }
        if self.capture.is_some_and(|capture| {
            !self
                .previous_hits
                .iter()
                .any(|h| h.id == capture.hit.id && h.action == capture.hit.action)
                && !matches!(capture.hit.action, super::HitAction::ColumnResize { table, column }
                    if self.tables.get(&table).is_some_and(|state| state.last_frame == self.frame && state.resize_columns.contains(&column))
                        && self.visible_windows.contains(&capture.hit.window))
        }) {
            self.capture = None;
            self.gesture_cancel();
        }
        self.splits
            .retain(|_, state| state.last_frame == self.frame);
        self.trees.retain(|_, state| state.last_frame == self.frame);
        self.tree_input.clear();
        self.finish_collapsing_headers();
        self.split_input.clear();
        self.clicked.clear();
        self.slider_input.clear();
        self.text_edit_input.clear();
        self.number_input.clear();
        self.numbers
            .retain(|_, state| state.last_frame == self.frame);
        self.combo_input.clear();
        self.combo_boxes
            .retain(|_, state| state.last_frame == self.frame);
        self.dismissed_popups
            .retain(|id| self.seen.contains(&id.with("body")));
        self.color_pickers
            .retain(|id, _| self.seen.contains(&id.with("swatch")));
        self.text_edits
            .retain(|id, _| self.seen.contains(&id.with("body")));
        self.input.finish_frame();
    }
}
