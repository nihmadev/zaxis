use std::hash::Hash;

use crate::{context::Paint, layout::LayoutCursor, Context, Id, Layout, Rect, Shape, Vec2};

use super::{HoverStyle, Response, Style, Widget};

mod flow;
mod scopes;
use flow::Flow;
pub(crate) use flow::FlowState;

/// A vertical or horizontal layout inside a window. Constructed by [`super::Window::show`].
pub struct Ui<'a> {
    pub(crate) context: &'a mut Context,
    pub(super) window: Id,
    pub(super) scope: Id,
    pub(super) sequence: u64,
    pub(super) clip: Rect,
    pub(super) layout: LayoutCursor,
    pub(super) enabled: bool,
    pub(super) backdrop_blur: f32,
    pub(super) hover_style: Option<HoverStyle>,
    pub(super) flow: Option<Flow>,
}

impl Ui<'_> {
    pub fn displayed_rect(&self, response: &Response) -> Rect {
        self.context.visual_rect(response.id, response.rect)
    }
    /// Add a widget, automatically scheduling a follow-up redraw on activation or change.
    pub fn add(&mut self, widget: impl Widget) -> Response {
        let response = self.layout_item(|ui| widget.ui(ui));
        if response.clicked || response.changed || response.submitted || response.lost_focus {
            self.context.request_repaint();
        }
        response
    }
    pub fn style(&self) -> &Style {
        self.context.style()
    }
    /// Current hover preset, including an enclosing `Hover` component's override.
    pub fn hover_style(&self) -> HoverStyle {
        self.hover_style.unwrap_or(self.style().hover_style)
    }
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
    pub fn add_enabled(&mut self, enabled: bool, widget: impl Widget) -> Response {
        self.add_enabled_ui(enabled, |ui| ui.add(widget))
    }
    /// Disable a group without changing its layout or bound values.
    /// Nested groups cannot re-enable a disabled parent.
    pub fn add_enabled_ui<R>(&mut self, enabled: bool, build: impl FnOnce(&mut Self) -> R) -> R {
        let previous = self.enabled;
        self.enabled &= enabled;
        let result = build(self);
        self.enabled = previous;
        result
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
            .visual_clips
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
        let result = build(self);
        self.scope = old_scope;
        self.sequence = old_sequence;
        result
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

    /// Inherited glass controls reveal the already blurred panel backdrop.
    /// Explicit overrides and controls in opaque panels still filter independently.
    pub(super) fn control_blur(&self, radius: Option<f32>) -> (f32, f32) {
        let blur = super::blur::normalize_radius(radius.unwrap_or(self.style().blur_radius));
        let filter = if radius.is_none() && self.backdrop_blur > 0.0 {
            0.0
        } else {
            blur
        };
        (blur, filter)
    }

    pub(super) fn next_id(&mut self, kind: &'static str) -> Id {
        let id = self.scope.with((kind, self.sequence));
        self.sequence += 1;
        id
    }

    pub(super) fn response(&self, id: Id, rect: Rect, interactive: bool) -> Response {
        let interactive = interactive && self.enabled;
        let visible = self.context.scroll_visible(self.window, rect, self.clip);
        Response {
            id,
            rect,
            hovered: self.context.hovered(id, self.window, rect, self.clip),
            pressed: interactive && visible && self.context.active(id),
            has_focus: interactive && visible && self.context.has_focus(id),
            focus_visible: interactive && visible && self.context.focus_visible(id),
            enabled: interactive,
            clicked: interactive && visible && self.context.clicked(id),
            changed: false,
            submitted: false,
            lost_focus: false,
        }
    }
}
