//! `DropTarget`: a region that accepts payloads of type `P`.
use super::{
    indicator::{self, Indicator},
    style::DragStyle,
    DragEffect, DropZones, Dropped, Insertion,
};
use crate::{
    context::{
        drag::state::{Hover, TargetInfo},
        file_drop::FileTarget,
        HitAction, HitRegion,
    },
    files::{FileFilter, PickedFile},
    Id, Rect, Response, Ui, Vec2,
};
use std::{any::Any, convert::Infallible, marker::PhantomData, sync::Arc};

/// What a target takes from files dragged in from the system.
struct FileAccept {
    filter: Arc<FileFilter>,
    max: usize,
}

/// A drop region for payloads of type `P`. `accept` decides by value; the type
/// is checked first, so the predicate only ever sees a `&P`.
///
/// The predicate runs once per pass while a drag is active, before the target
/// highlights, and again when the drop is delivered. Idle targets register
/// nothing and do no work.
pub struct DropTarget<P, F> {
    key: Id,
    accept: F,
    enabled: bool,
    zones: Option<DropZones>,
    passthrough: bool,
    effect: Option<DragEffect>,
    indent: f32,
    indicator: bool,
    style: DragStyle,
    files: Option<FileAccept>,
    payload: PhantomData<fn(&P)>,
}

/// Result of a target for one pass.
#[derive(Debug)]
pub struct DropOutput<R, P> {
    pub inner: R,
    pub response: Response,
    /// The application id the target was created with.
    pub id: Id,
    /// A drag is over this target (it won target selection).
    pub hovering: bool,
    /// Hovering and the payload passed the type and value checks.
    pub acceptable: bool,
    /// Hovering, but the type or value was turned down.
    pub rejected: bool,
    /// Pointer relative to the target's top-left corner while hovering.
    pub position: Option<Vec2>,
    /// Zone under the pointer when the target was given [`DropZones`].
    pub insertion: Option<Insertion>,
    /// The drop, delivered once, on the pass after the pointer was released.
    pub dropped: Option<Dropped<P>>,
    /// Files from the system are being dragged over this target (it won target selection).
    pub files_hovering: bool,
    /// Hovering files, and the target's filter passes at least one.
    pub files_acceptable: bool,
    /// Files dropped on this target from the system, once: those its filter passes, up to
    /// its limit, in the order the system listed them.
    pub dropped_files: Vec<PickedFile>,
}

impl<P: Any, F: Fn(&P) -> bool> DropTarget<P, F> {
    pub fn new(id: Id, accept: F) -> Self {
        Self {
            key: id,
            accept,
            enabled: true,
            zones: None,
            passthrough: false,
            effect: None,
            indent: 0.0,
            indicator: true,
            style: DragStyle::default(),
            files: None,
            payload: PhantomData,
        }
    }
    /// Also take files dragged in from the system that `filter` passes. The target highlights
    /// while they hover, in the style of an in-window drag, and reports the drop in
    /// [`DropOutput::dropped_files`]. Exactly one target takes a drop: the topmost under the
    /// position, and none under a modal. Cache the `Arc` to avoid an allocation per frame.
    pub fn accepts_files(mut self, filter: impl Into<Arc<FileFilter>>) -> Self {
        self.files = Some(FileAccept {
            filter: filter.into(),
            max: usize::MAX,
        });
        self
    }
    /// Take at most `max` files of one drop (see [`accepts_files`](Self::accepts_files)).
    pub fn max_files(mut self, max: usize) -> Self {
        if let Some(files) = &mut self.files {
            files.max = max;
        }
        self
    }
    /// A disabled target is skipped by hit testing and keeps its state.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Report before/inside/after zones and draw an insertion line.
    pub fn zones(mut self, zones: DropZones) -> Self {
        self.zones = Some(zones);
        self
    }
    /// Let the enclosing target take the drop when this one rejects the payload.
    /// By default a nested rejecting target blocks its parent.
    pub fn passthrough_rejected(mut self, passthrough: bool) -> Self {
        self.passthrough = passthrough;
        self
    }
    /// Cursor shown over this target instead of the source's effect.
    pub fn effect(mut self, effect: DragEffect) -> Self {
        self.effect = Some(effect);
        self
    }
    /// Where an insertion line starts, for example the indent of a tree level.
    #[track_caller]
    pub fn line_indent(mut self, indent: f32) -> Self {
        self.indent = crate::components::sanitize::non_negative("DropTarget::line_indent", indent)
            .unwrap_or(self.indent);
        self
    }
    /// Draw nothing and only report state; the application draws its own marker.
    pub fn indicator(mut self, draw: bool) -> Self {
        self.indicator = draw;
        self
    }
    pub fn style(mut self, style: DragStyle) -> Self {
        self.style.merge(&style);
        self
    }

    /// Wrap freshly built content; the target covers the content's bounds.
    pub fn show<R>(
        self,
        ui: &mut Ui<'_>,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> DropOutput<R, P> {
        if ui.flow.is_some() {
            return ui.layout_item(|ui| self.show(ui, build));
        }
        let id = ui.scope.with(("drop-target", self.key));
        let active = self.enabled && ui.enabled && ui.context.drag_running();
        let nests = active || (self.files.is_some() && self.enabled && ui.enabled);
        let depth = if nests { ui.context.drag_enter() } else { 0 };
        let (inner, size, placement) = ui.measure_effect(id.with("content"), true, build);
        if nests {
            ui.context.drag_exit();
        }
        let rect = Rect::from_min_size(ui.layout.cursor, size);
        let state = self.resolve(ui, id, rect, depth, active);
        let clip = ui.clip_rect();
        ui.context.place(placement, Vec2::ZERO, clip);
        ui.allocate_space(size);
        let response = ui.response(id, rect, self.enabled);
        self.finish(ui, rect, inner, response, state)
    }

    /// Make an already built widget a target.
    pub fn attach(self, ui: &mut Ui<'_>, response: Response) -> DropOutput<(), P> {
        let id = ui.scope.with(("drop-target", self.key));
        let active = self.enabled && ui.enabled && response.enabled && ui.context.drag_running();
        let depth = ui.context.drag.depth;
        let state = self.resolve(ui, id, response.rect, depth, active);
        self.finish(ui, response.rect, (), response, state)
    }

    fn resolve(&self, ui: &mut Ui<'_>, id: Id, rect: Rect, depth: u16, active: bool) -> State<P> {
        let mut state = State {
            hover: None,
            dropped: None,
            file_hover: None,
            files: Vec::new(),
            id,
        };
        if let (Some(accept), true) = (&self.files, self.enabled && ui.enabled) {
            let slot = ui.context.file_add_target(FileTarget {
                id,
                key: self.key,
                depth,
                filter: Arc::clone(&accept.filter),
                max: accept.max,
                passthrough: self.passthrough,
            });
            ui.context.register_hit(HitRegion {
                id: id.with("files"),
                window: ui.window,
                rect,
                clip: ui.clip,
                action: HitAction::FileDrop { slot },
            });
            state.file_hover = ui.context.files.hover_target.filter(|h| h.id == id);
            state.files = ui.context.file_claim(id).unwrap_or_default();
            if !state.files.is_empty() {
                ui.context.request_repaint();
            }
        }
        if active {
            let accepts = ui
                .context
                .drag
                .session
                .as_ref()
                .and_then(|session| session.payload.as_ref())
                .and_then(|payload| payload.get::<P>())
                .is_some_and(|payload| (self.accept)(payload));
            let slot = ui.context.drag_add_target(TargetInfo {
                id,
                key: self.key,
                depth,
                accepts,
                passthrough: self.passthrough,
                zones: self.zones,
                effect: self.effect,
            });
            ui.context.register_hit(HitRegion {
                id,
                window: ui.window,
                rect,
                clip: ui.clip,
                action: HitAction::DropTarget { slot },
            });
            state.hover = ui
                .context
                .drag
                .session
                .as_ref()
                .and_then(|session| session.hover)
                .filter(|hover| hover.id == id);
        }
        if self.enabled && ui.enabled {
            state.dropped = self.claim(ui, id);
        }
        state
    }

    /// Take the dropped payload if it was released over this target and still
    /// passes the type and value checks. A target that vanished or changed its
    /// mind leaves the drop unclaimed, and the session ends as cancelled.
    fn claim(&self, ui: &mut Ui<'_>, id: Id) -> Option<Dropped<P>> {
        let ended = ui
            .context
            .drag
            .ended
            .as_mut()
            .filter(|ended| ended.target.id == id && !ended.claimed)?;
        if !ended
            .payload
            .as_ref()
            .and_then(|payload| payload.get::<P>())
            .is_some_and(|payload| (self.accept)(payload))
        {
            return None;
        }
        let payload = ended.payload.take()?.take::<P>().ok()?;
        ended.claimed = true;
        let dropped = Dropped {
            source: ended.info.key,
            target: self.key,
            position: ended.position,
            local: ended.target.local,
            insertion: ended.target.insertion,
            effect: ended.target.effect.unwrap_or(ended.info.effect),
            payload,
        };
        ui.context.request_repaint();
        Some(dropped)
    }

    fn finish<R>(
        self,
        ui: &mut Ui<'_>,
        rect: Rect,
        inner: R,
        response: Response,
        state: State<P>,
    ) -> DropOutput<R, P> {
        let hover: Option<Hover> = state.hover;
        let file_hover = state.file_hover;
        let mut response = response;
        response.files_hovering = file_hover.is_some();
        response.files_dropped = !state.files.is_empty();
        if let (Some(hover), true) = (hover.or(file_hover), self.indicator) {
            indicator::paint(
                ui,
                &self.style,
                Indicator {
                    id: state.id,
                    rect,
                    hover,
                    zones: self.zones,
                    indent: self.indent,
                },
            );
        }
        DropOutput {
            inner,
            response,
            id: self.key,
            hovering: hover.is_some(),
            acceptable: hover.is_some_and(|h| h.accepts),
            rejected: hover.is_some_and(|h| !h.accepts),
            position: hover.map(|h| h.local),
            insertion: hover.and_then(|h| h.insertion),
            dropped: state.dropped,
            files_hovering: file_hover.is_some(),
            files_acceptable: file_hover.is_some_and(|h| h.accepts),
            dropped_files: state.files,
        }
    }
}

impl DropTarget<Infallible, fn(&Infallible) -> bool> {
    /// A target for files dragged in from the system only, taking every file; narrow it
    /// with [`accepts_files`](Self::accepts_files). It has no in-window payload.
    pub fn files(id: Id) -> Self {
        fn never(_: &Infallible) -> bool {
            false
        }
        Self::new(id, never as fn(&Infallible) -> bool)
            .accepts_files(FileFilter::any().directories(true))
    }
}

struct State<P> {
    hover: Option<Hover>,
    dropped: Option<Dropped<P>>,
    file_hover: Option<Hover>,
    files: Vec<PickedFile>,
    id: Id,
}
