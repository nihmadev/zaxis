use std::{hash::Hash, panic::Location};

mod chrome;
mod rows;
pub use chrome::ScrollStyle;

use super::Ui;
use crate::{
    context::scroll::{ScrollScope, ScrollState},
    layout::LayoutCursor,
    Id, Layout, Rect, Vec2,
};

/// A measured, clipped child UI with an offset retained by a scoped stable ID.
/// Scroll deltas remain fractional logical pixels; positive offsets reveal later content.
pub struct ScrollArea {
    axes: [bool; 2],
    id: Option<Id>,
    source: &'static Location<'static>,
    max_size: Vec2,
    content_width: Option<f32>,
    style: Option<ScrollStyle>,
    offset: Option<Vec2>,
    target: Option<Rect>,
    bars: bool,
    overlay_bars: bool,
    hints: bool,
    middle_mouse_scroll: bool,
}

pub struct ScrollAreaOutput<R> {
    pub inner: R,
    pub id: Id,
    /// The content viewport in screen coordinates, before ancestor clipping.
    pub viewport: Rect,
    pub content_size: Vec2,
    pub offset: Vec2,
}
impl ScrollArea {
    #[track_caller]
    pub fn vertical() -> Self {
        Self::new([false, true])
    }
    #[track_caller]
    pub fn horizontal() -> Self {
        Self::new([true, false])
    }
    #[track_caller]
    pub fn both() -> Self {
        Self::new([true, true])
    }
    #[track_caller]
    fn new(axes: [bool; 2]) -> Self {
        Self {
            axes,
            id: None,
            source: Location::caller(),
            max_size: Vec2::new(f32::INFINITY, 300.0),
            content_width: None,
            style: None,
            offset: None,
            target: None,
            bars: true,
            overlay_bars: false,
            hints: true,
            middle_mouse_scroll: true,
        }
    }
    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }
    pub fn id_source(self, source: impl Hash) -> Self {
        self.id(Id::new(source))
    }
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_size.x = length(width);
        self
    }
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_size.y = length(height);
        self
    }
    /// Layout width for horizontal overflow. Otherwise use the viewport width;
    /// unwrapped text and explicitly allocated geometry can still extend beyond it.
    pub fn content_width(mut self, width: f32) -> Self {
        self.content_width = Some(length(width));
        self
    }
    pub fn style(mut self, style: ScrollStyle) -> Self {
        self.style = Some(style);
        self
    }
    /// Set the offset on this pass. Omit on following passes to retain user scrolling.
    pub fn scroll_offset(mut self, offset: Vec2) -> Self {
        assert!(offset.is_finite(), "scroll offset must be finite");
        self.offset = Some(offset.max(Vec2::ZERO));
        self
    }
    /// Reveal a rectangle in content coordinates (zero is the content origin).
    pub fn scroll_to_rect(mut self, rect: Rect) -> Self {
        assert!(
            rect.min.is_finite() && rect.max.is_finite(),
            "scroll target must be finite"
        );
        self.target = Some(rect);
        self
    }
    /// Enable middle-button autoscroll (enabled by default).
    pub fn middle_mouse_scroll(mut self, enabled: bool) -> Self {
        self.middle_mouse_scroll = enabled;
        self
    }
    pub fn show_scrollbars(mut self, show: bool) -> Self {
        self.bars = show;
        self
    }
    /// Draw scrollbars over content without reserving a gutter (off by default).
    pub fn overlay_scrollbars(mut self, overlay: bool) -> Self {
        self.overlay_bars = overlay;
        self
    }
    pub fn show_hints(mut self, show: bool) -> Self {
        self.hints = show;
        self
    }

    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> ScrollAreaOutput<R> {
        self.show_content(ui, None, build)
    }

    fn show_content<R>(
        self,
        ui: &mut Ui<'_>,
        known_height: Option<f32>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> ScrollAreaOutput<R> {
        ui.layout_item(|ui| self.show_content_inner(ui, known_height, build))
    }

    fn show_content_inner<R>(
        self,
        ui: &mut Ui<'_>,
        known_height: Option<f32>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> ScrollAreaOutput<R> {
        let id = match self.id {
            Some(id) => ui.scope.with(("scroll", id)),
            None => ui.auto_id(("scroll", self.source)),
        };
        assert!(
            !ui.context.scrolling.order.contains(&id),
            "duplicate scroll ID"
        );
        let style = self.style.unwrap_or(ui.style().scroll);
        let size = Vec2::new(ui.available_width(), ui.available_height()).min(self.max_size);
        let outer = ui.allocate_space(size);
        let mut viewport = style.padding.inset(outer);
        let gutter = if self.bars && !self.overlay_bars {
            style.bar_width.max(0.0) + style.bar_margin.max(0.0)
        } else {
            0.0
        };
        viewport.max = (viewport.max
            - Vec2::new(
                if self.axes[1] { gutter } else { 0.0 },
                if self.axes[0] { gutter } else { 0.0 },
            ))
        .max(viewport.min);
        let parent = ui
            .context
            .scrolling
            .stack
            .last()
            .copied()
            .filter(|&n| ui.context.scrolling.scopes[n].window == ui.window);
        let parent_id = parent.map(|n| ui.context.scrolling.scopes[n].id);
        let state = ui
            .context
            .scrolling
            .states
            .entry(id)
            .or_insert_with(|| ScrollState::new(ui.window));
        state.viewport = viewport;
        state.visual_scale = 1.0;
        state.axes = self.axes;
        state.parent = parent_id;
        state.enabled = ui.enabled;
        state.middle_mouse_scroll = self.middle_mouse_scroll;
        state.last_frame = ui.context.frame;
        if let Some(height) = known_height {
            state.content.y = height;
        }
        if let Some(offset) = self.offset {
            state.offset = offset;
        }
        if let Some(target) = self.target {
            state.offset = reveal(state.offset, viewport.size(), target, self.axes);
        }
        // Ordinary closures measure after building. Do not clamp new targets against
        // the old content size; fixed-row content can be clamped before virtualization.
        if self.target.is_none() && self.offset.is_none() || known_height.is_some() {
            state.offset = state.offset.clamp(Vec2::ZERO, state.max_offset());
        }
        let offset = state.offset;
        let origin = viewport.min - offset;
        let width = self
            .content_width
            .unwrap_or(viewport.size().x)
            .max(viewport.size().x);
        let bounds = Rect::from_min_size(
            origin,
            Vec2::new(
                width,
                if self.axes[1] {
                    1.0e9
                } else {
                    viewport.size().y
                },
            ),
        );
        let scope = ui.context.scrolling.scopes.len();
        ui.context.scrolling.scopes.push(ScrollScope {
            visual_scale: 1.0,
            id,
            window: ui.window,
            parent,
            viewport,
            outer_clip: ui.clip,
            region: outer,
            correction: Vec2::ZERO,
            target: None,
            origin,
        });
        ui.context.scrolling.stack.push(scope);
        ui.context.scrolling.order.push(id);
        let mut child = Ui {
            flow: None,
            context: ui.context,
            window: ui.window,
            scope: id,
            sequence: 0,
            // Clip at emission, after measurement. Inner text/row clips remain in
            // content coordinates and can be translated without losing hidden edges.
            clip: Rect::from_min_size(Vec2::splat(-1.0e9), Vec2::splat(2.0e9)),
            layout: LayoutCursor::new(
                bounds,
                if self.axes == [true, false] {
                    Layout::Horizontal
                } else {
                    Layout::Vertical
                },
                style.spacing.max(0.0),
            ),
            enabled: ui.enabled,
            backdrop_blur: ui.backdrop_blur,
            hover_style: ui.hover_style,
            local_style: ui.local_style.clone(),
            local_style_revision: ui.local_style_revision,
        };
        if known_height.is_none() {
            child.begin_layout(crate::Align::Start);
        }
        let inner = build(&mut child);
        child.finish_layout();
        let mut content = child.layout.used;
        if let Some(height) = known_height {
            content.y = height;
        }
        let target = child.context.scrolling.scopes[scope].target;
        let state = child.context.scrolling.states.get_mut(&id).unwrap();
        state.content = content;
        if let Some(target) = target {
            state.offset = reveal(state.offset, viewport.size(), target, self.axes);
        }
        state.offset = state.offset.clamp(Vec2::ZERO, state.max_offset());
        let final_offset = state.offset;
        child.context.scrolling.scopes[scope].correction = offset - final_offset;
        if final_offset != offset {
            child.context.request_repaint();
        }
        child.context.end_scroll();
        self.paint_chrome(ui, id, outer, viewport, content, final_offset, style);
        ScrollAreaOutput {
            inner,
            id,
            viewport,
            content_size: content,
            offset: final_offset,
        }
    }
}
fn length(value: f32) -> f32 {
    assert!(
        value.is_finite() && value >= 0.0,
        "scroll dimensions must be finite and nonnegative"
    );
    value
}
fn reveal(mut offset: Vec2, size: Vec2, target: Rect, axes: [bool; 2]) -> Vec2 {
    for axis in 0..2 {
        if !axes[axis] {
            offset[axis] = 0.0;
            continue;
        }
        if target.min[axis] < offset[axis] || target.size()[axis] > size[axis] {
            offset[axis] = target.min[axis];
        } else if target.max[axis] > offset[axis] + size[axis] {
            offset[axis] = target.max[axis] - size[axis];
        }
    }
    offset.max(Vec2::ZERO)
}
impl Ui<'_> {
    /// Reveal an element/rectangle in screen coordinates in the nearest ScrollArea.
    /// The closure runs once; the measured paint and hits move together on this pass.
    pub fn scroll_to_rect(&mut self, rect: Rect) {
        assert!(
            rect.min.is_finite() && rect.max.is_finite(),
            "scroll target must be finite"
        );
        self.context.scroll_target(self.window, rect);
    }
    pub fn scroll_to_response(&mut self, response: &super::Response) {
        self.scroll_to_rect(response.rect);
    }
    #[track_caller]
    pub fn scroll_vertical<R>(
        &mut self,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> ScrollAreaOutput<R> {
        ScrollArea::vertical().show(self, build)
    }
}
