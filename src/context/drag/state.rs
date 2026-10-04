//! Retained drag state. Nothing here borrows from the application: payloads are
//! owned boxes, ids are plain values and no closure is stored.
use super::super::{HitRegion, Id, Paint};
use crate::components::drag_drop::{
    style::Resolved, DragEffect, DragEnd, DropZones, Insertion, PreviewKind,
};
use crate::{Rect, Vec2};
use std::{
    any::{Any, TypeId},
    time::Instant,
};
use winit::window::CursorIcon;

/// Application payload. The type is erased only for storage; targets compare
/// `TypeId` before they downcast, so a value is never reinterpreted.
pub(crate) struct Payload {
    pub type_id: TypeId,
    pub value: Box<dyn Any>,
}

impl Payload {
    pub fn new<P: Any>(value: P) -> Self {
        Self {
            type_id: TypeId::of::<P>(),
            value: Box::new(value),
        }
    }
    pub fn get<P: Any>(&self) -> Option<&P> {
        self.value.downcast_ref()
    }
    pub fn take<P: Any>(self) -> Result<P, Self> {
        if self.type_id != TypeId::of::<P>() {
            return Err(self);
        }
        match self.value.downcast::<P>() {
            Ok(value) => Ok(*value),
            Err(value) => Err(Self {
                type_id: self.type_id,
                value,
            }),
        }
    }
}

/// Registered by a source on every pass in which it is enabled. Hits refer to
/// it by index, which stays valid because both are replaced together.
#[derive(Clone, Copy)]
pub(crate) struct SourceInfo {
    /// Scoped id, also the id of the passive hit.
    pub id: Id,
    /// The application's id, reported in results.
    pub key: Id,
    pub depth: u16,
    /// Focus owner that picks the source up with the keyboard.
    pub focus: Option<Id>,
    /// The source owns a dedicated focus stop, so plain Space is enough.
    pub own_focus: bool,
    pub effect: DragEffect,
    pub preview: PreviewKind,
    pub resolved: Resolved,
}

#[derive(Clone, Copy)]
pub(crate) struct TargetInfo {
    pub id: Id,
    pub key: Id,
    pub depth: u16,
    pub accepts: bool,
    /// A rejecting target hands the drop to its enclosing target.
    pub passthrough: bool,
    pub zones: Option<DropZones>,
    pub effect: Option<DragEffect>,
}

/// The target and zone currently under the pointer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hover {
    pub id: Id,
    pub key: Id,
    pub accepts: bool,
    pub insertion: Option<Insertion>,
    pub local: Vec2,
    pub rect: Rect,
    pub effect: Option<DragEffect>,
}

/// What the source painted when the drag began, replayed under the pointer.
#[derive(Clone)]
pub(crate) struct Snapshot {
    pub rect: Rect,
    pub items: Vec<(Id, Vec<Paint>)>,
}

pub(crate) struct Pending {
    pub hit: HitRegion,
    pub info: SourceInfo,
    pub press: Vec2,
    pub time: Instant,
    /// Nothing interactive was under the press, so the source reports clicks.
    pub passive: bool,
}

pub(crate) struct Session {
    pub info: SourceInfo,
    pub window: Id,
    pub rect: Rect,
    pub grab: Vec2,
    pub pointer: Vec2,
    pub keyboard: bool,
    /// Pointer left the native window; nothing is under it until it moves again.
    pub outside: bool,
    pub payload: Option<Payload>,
    /// The payload was captured and the beginning reported.
    pub started: bool,
    /// The source (or its keep-alive) was seen on this pass.
    pub seen: bool,
    pub hover: Option<Hover>,
    pub cursor: CursorIcon,
    pub snapshot: Option<Snapshot>,
    pub last_tick: Option<Instant>,
}

pub(crate) struct Ended {
    pub info: SourceInfo,
    pub target: Hover,
    pub position: Vec2,
    pub payload: Option<Payload>,
    pub claimed: bool,
}

/// Preview flying back to its source after a cancelled drag.
pub(crate) struct Returning {
    pub snapshot: Snapshot,
    pub from: Vec2,
    pub to: Vec2,
    pub scale: f32,
    pub opacity: f32,
    pub tween: crate::TweenOptions,
    pub source: Id,
}

#[derive(Default)]
pub(crate) struct DragRuntime {
    pub pending: Option<Pending>,
    pub session: Option<Session>,
    pub ended: Option<Ended>,
    /// Result visible to sources from the given frame on, for one pass.
    pub result: Option<(DragEnd, u64)>,
    pub sources: Vec<SourceInfo>,
    pub targets: Vec<TargetInfo>,
    pub last_sources: Vec<SourceInfo>,
    pub last_targets: Vec<TargetInfo>,
    pub depth: u16,
    pub returning: Option<Returning>,
    /// A source built custom preview content on this pass.
    pub custom_preview: u64,
    pub deadline: Option<Instant>,
    pub scrolling: bool,
}

impl DragRuntime {
    pub(crate) fn active(&self) -> bool {
        self.session.as_ref().is_some_and(|s| s.started)
    }
}
