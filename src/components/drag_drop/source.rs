//! `DragSource`: makes already-built or freshly built content draggable.
use super::{
    preview,
    style::{DragStyle, Resolved},
    DragEffect, DragEnd, PreviewKind,
};
use crate::{
    context::{
        drag::{state::SourceInfo, Payload},
        HitAction, HitRegion,
    },
    Id, Rect, Response, Shape, Ui, Vec2,
};
use std::{any::Any, time::Duration};

type Custom<'a> = Box<dyn FnOnce(&mut Ui<'_>) + 'a>;

enum Value<'a, P> {
    Now(P),
    Lazy(Box<dyn FnOnce() -> P + 'a>),
}

/// A draggable region carrying an application-owned payload of type `P`.
///
/// The payload is handed over once, when the drag begins, and is owned by the
/// session until a target accepts it or the drag ends. Nothing is serialized
/// or cloned. Pass a cheap value (an id, an index), or use [`DragSource::lazy`]
/// to build it only when a drag actually starts.
pub struct DragSource<'a, P> {
    key: Id,
    value: Value<'a, P>,
    enabled: bool,
    effect: DragEffect,
    keyboard: bool,
    focus: Option<Id>,
    preview: PreviewKind,
    custom: Option<Custom<'a>>,
    style: DragStyle,
}

/// Result of a source for one pass.
#[derive(Debug)]
pub struct DragOutput<R> {
    pub inner: R,
    /// Hover, press and focus state of the draggable region.
    pub response: Response,
    /// The application id the source was created with.
    pub id: Id,
    /// True on the single pass in which the drag begins.
    pub started: bool,
    /// This source is being dragged.
    pub active: bool,
    /// Reported once, on the pass after the session ended.
    pub finished: Option<DragEnd>,
}

impl<'a, P: Any> DragSource<'a, P> {
    pub fn new(id: Id, payload: P) -> Self {
        Self::with_value(id, Value::Now(payload))
    }
    /// Build the payload only when a drag begins. The closure runs during the
    /// call to `show`/`attach` and is never stored.
    pub fn lazy(id: Id, payload: impl FnOnce() -> P + 'a) -> Self {
        Self::with_value(id, Value::Lazy(Box::new(payload)))
    }
    fn with_value(key: Id, value: Value<'a, P>) -> Self {
        Self {
            key,
            value,
            enabled: true,
            effect: DragEffect::Move,
            keyboard: true,
            focus: None,
            preview: PreviewKind::Snapshot,
            custom: None,
            style: DragStyle::default(),
        }
    }
    /// A disabled source keeps its state but cannot start a drag. Disabling the
    /// source being dragged ends the session as `SourceLost`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    pub fn effect(mut self, effect: DragEffect) -> Self {
        self.effect = effect;
        self
    }
    /// Pointer travel before a press becomes a drag (logical pixels).
    pub fn threshold(mut self, pixels: f32) -> Self {
        self.style = self.style.threshold(pixels);
        self
    }
    /// Hold time before movement may start the drag.
    pub fn delay(mut self, delay: Duration) -> Self {
        self.style = self.style.delay(delay);
        self
    }
    /// Keyboard pick-up: Space on a source with its own focus stop, Ctrl+Space
    /// on a source attached to another focusable control. On by default.
    pub fn keyboard(mut self, enabled: bool) -> Self {
        self.keyboard = enabled;
        self
    }
    /// Focus owner whose Ctrl+Space picks this source up. Defaults to the widget
    /// passed to `attach`; a composite widget such as a tree names its own owner.
    pub fn focus_owner(mut self, owner: Id) -> Self {
        self.focus = Some(owner);
        self
    }
    pub fn style(mut self, style: DragStyle) -> Self {
        self.style.merge(&style);
        self
    }
    pub fn hide_source(mut self, hide: bool) -> Self {
        self.style = self.style.hide_source(hide);
        self
    }
    pub fn no_preview(mut self) -> Self {
        self.preview = PreviewKind::None;
        self
    }
    /// Replace the snapshot with content built each pass at the pointer. The
    /// content is drawn in the overlay layer and never receives input.
    pub fn preview(mut self, build: impl FnOnce(&mut Ui<'_>) + 'a) -> Self {
        self.preview = PreviewKind::Custom;
        self.custom = Some(Box::new(build));
        self
    }

    /// Wrap freshly built content. The region grows with the content, keeps the
    /// content's own clicks and hover until the drag begins, and is dimmed while
    /// dragged.
    pub fn show<R>(self, ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> DragOutput<R> {
        if ui.flow.is_some() {
            return ui.layout_item(|ui| self.show(ui, build));
        }
        let id = ui.scope.with(("drag-source", self.key));
        let depth = ui.context.drag_enter();
        let enabled = self.enabled && ui.enabled;
        let focus = self.keyboard.then(|| id.with("focus"));
        if let (true, Some(focus)) = (enabled, focus) {
            ui.context.drag_reserve_order(focus);
        }
        let (inner, size, placement) = ui.measure_effect(id.with("content"), true, build);
        ui.context.drag_exit();
        let rect = Rect::from_min_size(ui.layout.cursor, size);
        if let (true, Some(focus)) = (enabled, focus) {
            // A dedicated focus stop below the content's own hits.
            ui.context.register_hit(HitRegion {
                id: focus,
                window: ui.window,
                rect,
                clip: ui.clip,
                action: HitAction::Focus,
            });
        }
        let resolved = self.style.resolve(ui.style());
        let key = self.key;
        let done = self.register(ui, id, rect, enabled, (focus, true), depth, resolved);
        let clip = ui.clip_rect();
        if done.dim < 1.0 {
            ui.context
                .place_visual(placement, crate::Transform::IDENTITY, done.dim, clip, true);
        } else {
            ui.context.place(placement, Vec2::ZERO, clip);
        }
        ui.allocate_space(size);
        let mut response = ui.response(id, rect, enabled);
        if let Some(focus) = focus.filter(|_| enabled) {
            response.has_focus = ui.context.has_focus(focus);
            response.focus_visible = ui.context.focus_visible(focus);
            if response.focus_visible {
                paint_focus(ui, id, rect);
            }
        }
        DragOutput {
            inner,
            response,
            id: key,
            started: done.started,
            active: done.active,
            finished: done.finished,
        }
    }

    /// Make an already built widget a source. The widget keeps focus and Space;
    /// use Ctrl+Space to pick it up with the keyboard. It is not dimmed.
    pub fn attach(self, ui: &mut Ui<'_>, response: Response) -> DragOutput<()> {
        let id = ui.scope.with(("drag-source", self.key));
        let enabled = self.enabled && ui.enabled && response.enabled;
        let focus = self.keyboard.then(|| self.focus.unwrap_or(response.id));
        let resolved = self.style.resolve(ui.style());
        let depth = ui.context.drag.depth;
        let key = self.key;
        let done = self.register(
            ui,
            id,
            response.rect,
            enabled,
            (focus, false),
            depth,
            resolved,
        );
        DragOutput {
            inner: (),
            response,
            id: key,
            started: done.started,
            active: done.active,
            finished: done.finished,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn register(
        self,
        ui: &mut Ui<'_>,
        id: Id,
        rect: Rect,
        enabled: bool,
        (focus, own_focus): (Option<Id>, bool),
        depth: u16,
        resolved: Resolved,
    ) -> Registered {
        let key = self.key;
        let mut done = Registered {
            started: false,
            active: false,
            dim: 1.0,
            finished: None,
        };
        if enabled {
            let slot = ui.context.drag_add_source(SourceInfo {
                id,
                key,
                depth,
                focus,
                own_focus,
                effect: self.effect,
                preview: self.preview,
                resolved,
            });
            ui.context.register_hit(HitRegion {
                id,
                window: ui.window,
                rect,
                clip: ui.clip,
                action: HitAction::DragSource { slot },
            });
            if let Some(session) = ui.context.drag_source_seen(id) {
                done.active = true;
                if session.payload.is_none() {
                    session.payload = Some(Payload::new(match self.value {
                        Value::Now(value) => value,
                        Value::Lazy(make) => make(),
                    }));
                    session.started = true;
                    done.started = true;
                }
                // Presentation follows the latest build of the source.
                session.info.preview = self.preview;
                session.info.effect = self.effect;
                session.info.resolved = resolved;
                done.dim = resolved.source_opacity;
            }
            if done.started {
                ui.context.request_repaint();
            }
            if done.active && self.preview == PreviewKind::Custom {
                if let Some(build) = self.custom {
                    preview::build_custom(ui, build);
                }
            }
        }
        done.finished = ui
            .context
            .drag
            .result
            .filter(|(end, from)| end.source == key && ui.context.frame >= *from)
            .map(|(end, _)| end);
        done
    }
}

struct Registered {
    started: bool,
    active: bool,
    dim: f32,
    finished: Option<DragEnd>,
}

fn paint_focus(ui: &mut Ui<'_>, id: Id, rect: Rect) {
    let style = ui.style();
    let shape = Shape::rect(rect, crate::Color::TRANSPARENT)
        .corner_radius(style.rounding)
        .border(style.focus_border);
    ui.context.paint(
        id.with("focus-ring"),
        ui.window,
        ui.clip,
        vec![crate::context::Paint::Shape(shape.into())],
    );
}
