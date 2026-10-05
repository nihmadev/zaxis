use super::*;
use crate::{
    context::{HitAction, HitRegion},
    layout::LayoutCursor,
    Align,
};

impl SplitPane {
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut SplitUi<'_, '_>) -> R,
    ) -> SplitOutput<R> {
        ui.layout_item(|ui| self.show_content(ui, build))
    }
    fn show_content<R>(
        mut self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut SplitUi<'_, '_>) -> R,
    ) -> SplitOutput<R> {
        let id = ui.scope.with(("split", self.id));
        let mut style = self.style.unwrap_or(ui.style().split);
        if let Some(gap) = self.gap {
            style.gap = gap;
        }
        if let Some(handle) = self.handle {
            style.handle = handle;
        }
        style.gap = allocation::dimension(style.gap);
        style.spacing = allocation::dimension(style.spacing);
        style.keyboard_step = allocation::dimension(style.keyboard_step);
        style.keyboard_large_step = allocation::dimension(style.keyboard_large_step);
        style.container = paint::normalized_surface(style.container);
        style.panel = paint::normalized_surface(style.panel);
        style.handle = paint::normalized_handle(style.handle);
        let mut unique = std::collections::HashSet::new();
        for p in &mut self.panels {
            if !unique.insert(p.id) {
                ui.context
                    .report(crate::DiagnosticKind::IdCollision, Some(p.id), None, || {
                        "duplicate SplitPanel id".into()
                    });
            }
            p.initial = p.initial.normalized();
            p.controlled = p.controlled.map(SplitSize::normalized);
            p.minimum = allocation::dimension(p.minimum);
            p.maximum = p.maximum.map(allocation::dimension);
            p.surface = p.surface.map(paint::normalized_surface);
            p.handle = p.handle.map(paint::normalized_handle);
        }
        let available = Vec2::new(ui.available_width(), ui.available_height());
        let rect = ui.allocate_space(self.size.unwrap_or(available).min(available));
        paint::surface(ui, id, rect, ui.clip, style.container);
        let bounds = paint::content_bounds(rect, style.container);
        let total = self.axis.main(bounds.size());
        let count = self.panels.len().saturating_sub(1);
        let gap = if count == 0 {
            0.0
        } else {
            style.gap.min(total / count as f32)
        };
        let budget = (total - gap * count as f32).max(0.0);
        let mut state = ui.context.splits.states.remove(&id).unwrap_or_default();
        state.last_frame = ui.context.frame;
        state.prefs.retain(|id, _| unique.contains(id));
        if self.reset {
            state.prefs.clear();
            state.allocation = None;
            if let Some(d) = state.drag.as_ref() {
                ui.context.cancel_split_capture(d.id);
            }
        }
        let prefs: Vec<_> = self
            .panels
            .iter()
            .map(|p| {
                p.controlled
                    .or_else(|| state.prefs.get(&p.id).copied())
                    .unwrap_or(p.initial)
            })
            .collect();
        let allocation_key = allocation::key(&self.panels, &prefs, budget);
        let mut sizes = state
            .allocation
            .as_ref()
            .filter(|(key, _)| *key == allocation_key)
            .map_or_else(
                || allocation::resolve(&self.panels, &prefs, budget),
                |(_, sizes)| sizes.clone(),
            );
        state.allocation = Some((allocation_key, sizes.clone()));
        let (mut boundaries, ended) = interaction::inputs(
            ui,
            id,
            self.axis,
            &self.panels,
            &mut sizes,
            &mut state,
            style,
            self.resizable && ui.enabled,
            self.double_click_reset,
        );
        if boundaries.iter().any(|b| b.changed)
            && self.panels.iter().all(|p| p.controlled.is_none())
        {
            // Keep the exact accepted pair allocation, rather than converting
            // weights back into sizes on every pass. Repeated f32 round trips
            // can move untouched edges across a scissor pixel boundary.
            let prefs: Vec<_> = self
                .panels
                .iter()
                .map(|p| state.prefs.get(&p.id).copied().unwrap_or(p.initial))
                .collect();
            state.allocation = Some((allocation::key(&self.panels, &prefs, budget), sizes.clone()));
        }
        let changed = self.panels.len() != state.sizes.len()
            || self
                .panels
                .iter()
                .zip(&sizes)
                .any(|(p, size)| state.sizes.get(&p.id) != Some(size));
        state.sizes = self
            .panels
            .iter()
            .zip(&sizes)
            .map(|(p, &size)| (p.id, size))
            .collect();
        state.axis = Some(self.axis);
        let mut panels = Vec::new();
        // Compute cumulative endpoints in f64. The last panel ends exactly at bounds.max.
        let mut offset = 0.0_f64;
        for (i, (p, &size)) in self.panels.iter().zip(&sizes).enumerate() {
            let start = (offset as f32).min(total);
            offset += size as f64;
            let end = if i + 1 == self.panels.len() {
                total
            } else {
                (offset as f32).min(total)
            };
            let outer = Rect::from_min_size(
                bounds.min + self.axis.size(start, 0.0),
                self.axis
                    .size((end - start).max(0.0), self.axis.cross(bounds.size())),
            )
            .intersect(bounds);
            let surface = p.surface.unwrap_or(style.panel);
            paint::surface(ui, id.with(p.id), outer, ui.clip.intersect(bounds), surface);
            panels.push(SplitPanelOutput {
                id: p.id,
                bounds: outer,
                content_bounds: paint::content_bounds(outer, surface),
                size: self.axis.main(outer.size()),
            });
            offset += gap as f64;
        }
        let handles = access::Handles::new(ui, self.axis, &self.panels, &sizes, &boundaries, style);
        let mut split = SplitUi {
            ui,
            id,
            specs: &self.panels,
            outputs: &panels,
            style,
            built: std::collections::HashSet::new(),
            handles,
        };
        let inner = build(&mut split);
        let SplitUi {
            ui, mut handles, ..
        } = split;
        // Boundaries win in their narrow zone, including panel-edge mode. Popups
        // still have a higher layer, so their controls keep portal precedence.
        for (i, out) in boundaries.iter_mut().enumerate() {
            let h = self.panels[i].handle.unwrap_or(style.handle);
            out.bounds =
                paint::hit_rect(self.axis, panels[i].bounds, panels[i + 1].bounds, bounds, h);
            if out.enabled {
                ui.context.register_hit(HitRegion {
                    id: out.id,
                    window: ui.window,
                    rect: out.bounds,
                    clip: ui.clip.intersect(bounds),
                    action: HitAction::SplitResize {
                        vertical: self.axis == Layout::Vertical,
                    },
                });
            }
            handles.place(ui, i, out.bounds, ui.clip.intersect(bounds));
            out.hovered = out.enabled && ui.context.hovered(out.id, ui.window, out.bounds, ui.clip);
            out.focused = out.enabled && ui.context.has_focus(out.id);
            paint::handle(
                ui,
                out.id,
                self.axis,
                out.bounds,
                h,
                out.hovered,
                out.enabled && ui.context.focus_visible(out.id),
                out.enabled && ui.context.active(out.id),
            );
        }
        let resize_started = boundaries.iter().any(|b| b.resize_started);
        let resize_ended = ended || boundaries.iter().any(|b| b.resize_ended);
        if boundaries.iter().any(|b| b.changed) || resize_started || resize_ended {
            ui.context.request_repaint();
        }
        ui.context.splits.states.insert(id, state);
        SplitOutput {
            inner,
            rect,
            panels,
            boundaries,
            changed,
            resize_started,
            resize_ended,
        }
    }
}
impl SplitUi<'_, '_> {
    pub fn panel<R>(&mut self, source: impl Hash, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.panel_id(Id::new(source), build)
    }
    pub fn panel_id<R>(&mut self, id: Id, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let known = self.specs.iter().position(|p| p.id == id);
        let unique = known.is_some() && self.built.insert(id);
        if !unique {
            let (kind, message) = if known.is_none() {
                (crate::DiagnosticKind::InvalidUsage, "unknown SplitPanel id")
            } else {
                (
                    crate::DiagnosticKind::IdCollision,
                    "duplicate id: SplitPanel content built more than once in a pass",
                )
            };
            self.ui
                .context
                .report(kind, Some(id), None, || message.into());
        }
        // A mistaken panel gets an empty area, so its content takes no space or input.
        let (bounds, surface) = match known.filter(|_| unique) {
            Some(i) => (
                self.outputs[i].content_bounds,
                self.specs[i].surface.unwrap_or(self.style.panel),
            ),
            None => (Rect::default(), self.style.panel),
        };
        let clip = self.ui.clip.intersect(bounds);
        // The tree reads panel, boundary, panel: the boundary before this panel comes first.
        let group = match known.filter(|_| unique) {
            Some(index) => {
                if let Some(before) = index.checked_sub(1) {
                    self.handles.describe(self.ui, before);
                }
                let scope = self.id.with(("panel", id));
                self.ui.a11y_begin(scope, crate::AccessRole::Group, |_| {})
            }
            None => crate::accessibility::Scope::NONE,
        };
        self.ui.context.begin_placement(self.ui.window);
        self.ui.context.visuals.clips.push((self.ui.window, clip));
        let mut child = Ui {
            context: self.ui.context,
            window: self.ui.window,
            scope: self.id.with(("panel", id)),
            sequence: 0,
            clip,
            layout: LayoutCursor::new(bounds, Layout::Vertical, self.style.spacing),
            enabled: self.ui.enabled,
            backdrop_blur: if surface.blur > 0.0 {
                surface.blur
            } else {
                self.ui.backdrop_blur
            },
            hover_style: self.ui.hover_style,
            local_style: self.ui.local_style.clone(),
            local_style_revision: self.ui.local_style_revision,
            flow: None,
        };
        child.begin_layout(Align::Start);
        let result = build(&mut child);
        child.finish_layout();
        child.context.visuals.clips.pop();
        let placement = child.context.end_placement();
        child.context.place(placement, Vec2::ZERO, clip);
        if let Some(index) = known.filter(|_| group.0.is_some()) {
            self.ui.a11y_end(group, Some(self.outputs[index].bounds));
        }
        result
    }
    /// Bounds of a panel; an unknown id gives an empty rectangle.
    pub fn bounds(&self, source: impl Hash) -> Rect {
        let id = Id::new(source);
        self.outputs
            .iter()
            .find(|p| p.id == id)
            .map_or(Rect::default(), |p| p.bounds)
    }
}
