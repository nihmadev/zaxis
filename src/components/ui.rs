use std::hash::Hash;

use crate::{
    context::{HitAction, HitRegion, Paint},
    layout::LayoutCursor,
    Context, Id, Layout, Rect, Shape, Vec2,
};

use super::{HoverStyle, Response, Sense, Style, Widget};

mod flow;
mod scopes;
mod theme;
use flow::Flow;
pub(crate) use flow::FlowState;

/// A vertical or horizontal layout inside a window. Constructed by [`super::Window::show`].
pub struct Ui<'a> {
    pub(crate) context: &'a mut Context,
    pub(crate) window: Id,
    pub(crate) scope: Id,
    pub(super) sequence: u64,
    pub(crate) clip: Rect,
    pub(super) layout: LayoutCursor,
    pub(crate) enabled: bool,
    pub(super) backdrop_blur: f32,
    pub(super) hover_style: Option<HoverStyle>,
    pub(super) flow: Option<Flow>,
    pub(super) local_style: Option<std::sync::Arc<Style>>,
    pub(super) local_style_revision: u64,
}

impl Ui<'_> {
    pub fn displayed_rect(&self, response: &Response) -> Rect {
        self.context.visual_rect(response.id, response.rect)
    }
    /// Add a widget, automatically scheduling a follow-up redraw on activation or change.
    pub fn add(&mut self, widget: impl Widget) -> Response {
        let response = self.layout_item(|ui| widget.ui(ui));
        if response.has_event() {
            self.context.request_repaint();
        }
        response
    }
    pub fn style(&self) -> &Style {
        self.local_style
            .as_deref()
            .unwrap_or_else(|| self.context.style())
    }
    /// Current hover preset, including an enclosing `Hover` component's override.
    pub fn hover_style(&self) -> HoverStyle {
        self.hover_style.unwrap_or(self.style().hover_style)
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    #[deprecated(
        note = "use `add_enabled_ui(enabled, |ui| ui.add(widget))`, or `.enabled(..)` on the widget"
    )]
    pub fn add_enabled(&mut self, enabled: bool, widget: impl Widget) -> Response {
        self.add_enabled_ui(enabled, |ui| ui.add(widget))
    }
    /// The one way to disable a group of controls without changing layout or
    /// bound values; the one way for a single control is its `.enabled(bool)`
    /// builder. Nested groups cannot re-enable a disabled parent.
    pub fn add_enabled_ui<R>(&mut self, enabled: bool, build: impl FnOnce(&mut Self) -> R) -> R {
        let previous = self.enabled;
        self.enabled &= enabled;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(self)));
        self.enabled = previous;
        match result {
            Ok(v) => v,
            Err(e) => std::panic::resume_unwind(e),
        }
    }
    pub fn context(&mut self) -> &mut Context {
        self.context
    }
    pub fn available_height(&self) -> f32 {
        (self.layout.bounds.max.y - self.layout.cursor.y).max(0.0)
    }

    pub fn available_width(&self) -> f32 {
        self.layout.available_width()
    }
    pub fn clip_rect(&self) -> Rect {
        let clip = self.context.scroll_clip(self.window, self.clip);
        self.context
            .visuals
            .clips
            .iter()
            .rev()
            .find(|(window, _)| *window == self.window)
            .map_or(clip, |(_, parent)| clip.intersect(*parent))
    }
    pub fn add_space(&mut self, amount: f32) {
        self.finish_layout_item();
        if let Some(flow) = &mut self.flow {
            flow.space(self.layout.direction, amount);
        }
        self.layout.space(amount);
    }

    /// Scope repeated widget labels, for example with a stable row ID in a loop.
    pub fn push_id<R>(&mut self, source: impl Hash, build: impl FnOnce(&mut Self) -> R) -> R {
        let old_scope = self.scope;
        let old_sequence = self.sequence;
        self.scope = self.scope.with(source);
        self.sequence = 0;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| build(self)));
        self.scope = old_scope;
        self.sequence = old_sequence;
        match result {
            Ok(v) => v,
            Err(e) => std::panic::resume_unwind(e),
        }
    }

    /// Lay out a nested row and allocate its occupied height in the parent layout.
    pub fn horizontal<R>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.horizontal_aligned(crate::Align::Start, build)
    }

    /// Lay out a nested column, for example alongside another column in a row.
    pub fn vertical<R>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.vertical_aligned(crate::Align::Start, build)
    }

    /// Allocate space for custom geometry, using the current layout direction.
    pub fn allocate_space(&mut self, size: Vec2) -> Rect {
        self.finish_layout_item();
        let rect = self.layout.allocate(size);
        if let Some(flow) = &mut self.flow {
            self.context.begin_placement(self.window);
            flow.pending = Some(rect);
        }
        rect
    }

    /// Paint a cached vector shape in logical coordinates, clipped to this UI.
    pub fn paint(&mut self, shape: impl Into<Shape>) {
        let id = self.next_id("shape");
        self.context
            .paint(id, self.window, self.clip, vec![Paint::Shape(shape.into())]);
    }

    pub(super) fn auto_id(&mut self, source: impl Hash) -> Id {
        let source = self.scope.with(source);
        let occurrence = self.context.auto_ids.entry(source).or_default();
        let id = source.with(*occurrence);
        *occurrence += 1;
        id
    }

    /// Global blur belongs to background surfaces. Controls stay opaque unless
    /// their own builder or typed surface explicitly requests backdrop blur.
    pub(super) fn control_blur(&self, radius: Option<f32>) -> (f32, f32) {
        let radius = super::blur::normalize_radius(radius.unwrap_or(0.0));
        (radius, radius)
    }

    pub(super) fn resolved_blur(&self, radius: f32, _explicit: bool) -> (f32, f32) {
        let radius = super::blur::normalize_radius(radius);
        (radius, radius)
    }

    pub(super) fn next_id(&mut self, kind: &'static str) -> Id {
        let id = self.scope.with((kind, self.sequence));
        self.sequence += 1;
        id
    }

    pub(super) fn response(&self, id: Id, rect: Rect, interactive: bool) -> Response {
        let interactive = interactive && self.enabled;
        let visible = interactive && self.context.scroll_visible(self.window, rect, self.clip);
        let events = if visible {
            self.context.gestures.events(id)
        } else {
            Default::default()
        };
        Response {
            id,
            rect,
            hovered: self.context.hovered(id, self.window, rect, self.clip),
            pressed: visible && self.context.active(id),
            has_focus: visible && self.context.has_focus(id),
            focus_visible: visible && self.context.focus_visible(id),
            enabled: interactive,
            clicked: visible && self.context.clicked(id),
            changed: false,
            submitted: false,
            lost_focus: events.lost_focus,
            gained_focus: events.gained_focus,
            double_clicked: events.double_clicked,
            secondary_clicked: events.secondary_clicked,
            drag_started: events.drag_started,
            dragging: events.dragging,
            drag_stopped: events.drag_stopped,
            drag_delta: events.drag_delta,
            menu_selected: None,
            link: None,
        }
    }

    /// Let a press-and-drag on `rect` move the enclosing [`super::Window`], for windows
    /// built with `title_bar(false)`. The region sits in the content layer like any other
    /// hit region: controls built after it win over it, and clipping and disabled groups
    /// apply. A [`super::Root`] cannot be moved; the call is then inert.
    pub fn drag_window(&mut self, rect: Rect) {
        let id = self.next_id("drag-window");
        if self.enabled {
            self.context.register_hit(HitRegion {
                id,
                window: self.window,
                rect,
                clip: self.clip,
                action: HitAction::Move,
            });
        }
    }

    /// The `Id` that [`Self::interact`] gives `id_source` in the current scope.
    pub(super) fn interact_id(&self, id_source: impl Hash) -> Id {
        self.scope.with(("interact", Id::new(id_source)))
    }

    /// Response for custom geometry, through the same hit path as built-in controls.
    ///
    /// Allocate `rect` with [`Self::allocate_space`] (or take it from your own layout)
    /// and give the widget a stable `id_source`. Clipping, scrolling, disabled groups,
    /// pointer capture, layer order, popups, keyboard activation and repaint behave
    /// exactly as they do for a [`super::Button`]; `sense` selects the inputs reported.
    /// A focusable region receives raw keys through [`Context::input`].
    pub fn interact(&mut self, rect: Rect, id_source: impl Hash, sense: Sense) -> Response {
        let id = self.interact_id(id_source);
        let response = self.response(id, rect, self.enabled);
        if sense.claims_pointer() || !self.enabled {
            self.context.register_hit(HitRegion {
                id,
                window: self.window,
                rect,
                clip: self.clip,
                action: if self.enabled {
                    HitAction::Interact(sense)
                } else {
                    HitAction::Block
                },
            });
        }
        if response.has_event() {
            self.context.request_repaint();
        }
        response
    }
}
