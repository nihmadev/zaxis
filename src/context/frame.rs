//! UI pass lifecycle: the order in which a pass drives every subsystem.
//!
//! Begin: clock, shared resources and theme, then each subsystem's per-pass data. The user
//! build runs once. Finish, in this order: overlays (stale popup, modals, tooltips, toasts,
//! drag preview), diagnostics, layer order of paint and hits, accessibility geometry,
//! scrolling and images, frame geometry; then what the pass built is published to route
//! the next input, caches are retired, focus and capture settle against the published
//! regions, retained widget state and routed input that nobody took are dropped, and the
//! accessibility tree and input state finish last.

use super::Context;
use crate::time::Instant;

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
        self.begin_pass(now);
        build(self);
        self.finish_frame();
        self.in_pass = false;
        true
    }

    fn begin_pass(&mut self, now: Instant) {
        self.frame_time = now.max(self.frame_time);
        self.in_pass = true;
        self.animations.begin_pass();
        if self
            .next_repaint
            .is_some_and(|time| time <= self.frame_time)
        {
            self.next_repaint = None;
        }
        self.sync_appearance();
        self.poll_clipboard();
        // Clear before the callback so repaint requests made by widgets survive it.
        self.dirty = false;
        self.ime_area = None;
        self.frame += 1;
        self.sample_theme_palette();
        self.images.lock().begin_frame(self.frame_time);
        self.text.begin_frame();
        self.stats.ui_passes += 1;
        self.paint_state.begin_pass();
        self.interaction.begin_pass();
        self.native_chrome = None;
        self.visuals.begin_pass();
        self.auto_ids.clear();
        self.popups.begin_pass();
        self.modals.begin_pass();
        self.tick_auto_scroll();
        self.drag_begin_frame();
        self.tick_selection_autoscroll();
        self.scrolling.begin_frame();
        self.carousel_wheel.begin_frame();
        self.a11y.begin_pass();
    }

    pub(super) fn finish_frame(&mut self) {
        // Events were delivered to this pass; anything recorded from here on
        // (vanished widgets, cancelled captures) belongs to the next one.
        self.gestures.finish_frame();
        self.finish_overlays();
        #[cfg(feature = "accesskit")]
        self.audit_accessibility();
        self.finish_diagnostics();
        self.order_by_layer();
        if self.a11y.active {
            self.a11y_resolve_geometry();
        }
        self.interaction.drop_unreachable();
        self.scrolling.finish_frame(self.frame);
        self.finish_auto_scroll();
        self.finish_images();
        self.rebuild_geometry();
        self.images_epoch = self.images.lock().epoch();
        self.text.end_frame();
        self.animations.finish_pass(self.frame);
        self.publish_routing();
        self.drag_finish_state();
        self.paint_state.retire(self.frame);
        self.settle_focus();
        self.finish_selection();
        self.retire_widget_state();
        self.finish_routed_input();
        #[cfg(feature = "accesskit")]
        self.finish_accessibility();
        self.a11y.end_pass(self.frame);
        self.input.finish_frame();
    }

    /// Close a popup that was not built in this pass or whose owner window was not, note
    /// the windows that were, then finish modals, tooltips, toasts and the drag preview.
    fn finish_overlays(&mut self) {
        let (frame, windows) = (self.frame, &self.windows);
        if self.popups.current.as_ref().is_some_and(|popup| {
            popup.last_frame != frame
                || !windows
                    .get(&popup.owner)
                    .is_some_and(|w| w.last_frame == frame)
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
        self.finish_toasts();
        self.drag_emit_preview();
    }

    /// Elements and hits in layer order (windows, then popup-class layers), each kept in
    /// paint or registration order within a layer.
    fn order_by_layer(&mut self) {
        let ranks: std::collections::HashMap<_, _> = self
            .layers
            .iter()
            .chain(self.popups.layers.iter())
            .copied()
            .map(|id| (id, self.layer_rank(id)))
            .collect();
        self.paint_state.sort(&ranks);
        self.interaction.sort(&ranks);
    }

    /// What this pass built routes the next input: the transforms of widgets that painted
    /// or registered a region, carousel wheel targets, hit regions and Tab registrations.
    fn publish_routing(&mut self) {
        let (paint, hits) = (&self.paint_state, &self.interaction.hits);
        self.visuals
            .publish(|id| paint.painted(id) || hits.iter().any(|hit| hit.id == id));
        self.carousel_wheel.finish_frame();
        self.interaction.publish();
        self.text_fields.publish_tabs();
    }

    /// Retained state of widgets not built in this pass is dropped, each family by its own
    /// rule. Popup dismissals, color pickers and text fields live while their body (or
    /// swatch) is painted. Tables and scroll areas keep their state while hidden.
    fn retire_widget_state(&mut self) {
        let frame = self.frame;
        self.visuals.retire_effects(frame);
        self.containers.retire(frame);
        self.local_styles.retain(|_, style| style.2 == frame);
        self.splits.retire(frame);
        self.trees.retire(frame);
        self.menus.retire(frame);
        self.values.retire_numbers(frame);
        let paint = &self.paint_state;
        self.popups.retire(|id| paint.painted(id));
        self.values.retire_color_pickers(|id| paint.painted(id));
        self.text_fields.retire(|id| paint.painted(id));
    }

    /// Routed input nobody took is dropped, after focus settled: focus changes for
    /// vanished fields go with it. Clicks that assistive technology delivered late (after
    /// scrolling their widget into view) belong to the next pass.
    fn finish_routed_input(&mut self) {
        self.trees.clear_input();
        self.splits.clear_input();
        self.interaction.clicked.clear();
        for id in self.a11y.take_late_clicks() {
            self.interaction.clicked.insert(id);
            self.request_repaint();
        }
        self.values.clear_queues();
        self.text_fields.clear_queue();
        self.menus.clear_keys();
    }
}
