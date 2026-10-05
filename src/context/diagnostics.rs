//! Usage diagnostics: problems in how the library is driven, found while the
//! UI is built. Detection is free in steady state (it only records when
//! something is wrong); reporting is a query on [`Context`], an optional
//! handler, and an opt-in overlay. Nothing is logged or printed.

use super::{Context, HitAction, HitRegion, Id};
use crate::Rect;
use std::{
    cell::RefCell,
    collections::HashSet,
    hash::{Hash, Hasher},
    panic::Location,
};

const MAX_PER_PASS: usize = 64;
const MAX_REMEMBERED: usize = 1024;

/// What went wrong. New kinds may be added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DiagnosticKind {
    /// Two interactive widgets used the same [`Id`] in one pass.
    IdCollision,
    /// A scope (scroll area, placement, visual group) was left open at the end of a pass.
    UnbalancedScope,
    /// An interactive widget was given an empty or non-finite rect.
    NoLayoutSpace,
    /// A popup had a non-finite anchor or no room to open.
    PopupWithoutAnchor,
    /// A builder argument was out of range and was replaced by a valid value.
    InvalidValue,
    /// A data model passed to a component is inconsistent, for example a tree cycle.
    InvalidModel,
    /// An API contract was broken and the call was made harmless, for example an
    /// extra Grid cell or an unknown panel id.
    InvalidUsage,
    /// A control that a screen reader announces by name has none. Found while the
    /// accessibility tree is collected; see `Context::set_accessibility_active`.
    MissingAccessibleName,
    /// A system resource failed and the app would otherwise never know: the
    /// clipboard, a native window drag or resize, an image that could not be decoded.
    External,
}

/// One usage problem. `rect` is where it happened when that is known.
#[derive(Clone, Debug, PartialEq)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub id: Option<Id>,
    pub rect: Option<Rect>,
    pub message: String,
}

/// Debug drawing above all layers; everything is off by default and costs
/// nothing while off. Enabling it never changes layout, hit testing or the
/// geometry cache of the application's own elements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DebugOverlay {
    /// A frame and a short message on the rect of every current diagnostic.
    pub issues: bool,
    /// Bounds of the topmost hit region under the cursor.
    pub bounds: bool,
    /// Clip rectangle of that region.
    pub clip: bool,
    /// Every hit region under the cursor, with its kind.
    pub hits: bool,
}

impl DebugOverlay {
    pub const OFF: Self = Self {
        issues: false,
        bounds: false,
        clip: false,
        hits: false,
    };
    pub const ISSUES: Self = Self {
        issues: true,
        ..Self::OFF
    };
    pub const ALL: Self = Self {
        issues: true,
        bounds: true,
        clip: true,
        hits: true,
    };
    pub(crate) fn active(self) -> bool {
        self != Self::OFF
    }
}

type Handler = Box<dyn FnMut(&Diagnostic)>;

#[derive(Default)]
pub(crate) struct Diagnostics {
    current: Vec<Diagnostic>,
    pub(super) published: Vec<Diagnostic>,
    remembered: HashSet<u64>,
    handler: Option<Handler>,
    pub(super) overlay: DebugOverlay,
}

struct Invalid {
    what: &'static str,
    detail: String,
    at: &'static Location<'static>,
}
thread_local! {
    static INVALID: RefCell<Vec<Invalid>> = const { RefCell::new(Vec::new()) };
}

/// Record a builder argument that was replaced; delivered by the next pass.
#[track_caller]
pub(crate) fn invalid_value(what: &'static str, detail: String) {
    let at = Location::caller();
    INVALID.with(|list| {
        let mut list = list.borrow_mut();
        if list.len() < MAX_PER_PASS {
            list.push(Invalid { what, detail, at });
        }
    });
}

impl Context {
    /// Problems found during the last completed pass. Empty when the UI is
    /// used correctly. Out-of-range builder values appear here too, for as
    /// long as they are still passed in.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics.published
    }

    /// Called once for each distinct problem, the first time it is seen, so a
    /// misconfigured widget does not report every frame. Replaces the previous handler.
    pub fn set_diagnostic_handler(&mut self, handler: Option<Handler>) {
        self.diagnostics.handler = handler;
    }

    pub fn set_debug_overlay(&mut self, overlay: DebugOverlay) {
        if self.diagnostics.overlay != overlay {
            self.diagnostics.overlay = overlay;
            self.request_repaint();
        }
    }

    pub fn debug_overlay(&self) -> DebugOverlay {
        self.diagnostics.overlay
    }

    pub(crate) fn report(
        &mut self,
        kind: DiagnosticKind,
        id: Option<Id>,
        rect: Option<Rect>,
        message: impl FnOnce() -> String,
    ) {
        let list = &mut self.diagnostics.current;
        if list.len() >= MAX_PER_PASS
            || list
                .iter()
                .any(|d| d.kind == kind && d.id == id && id.is_some())
        {
            return;
        }
        list.push(Diagnostic {
            kind,
            id,
            rect,
            message: message(),
        });
    }

    /// Final hit regions of controls that were laid out with no area. Measuring
    /// passes never reach this list, so only widgets that really stayed empty show up.
    pub(super) fn report_empty_hits(&mut self) {
        let mut empty = Vec::new();
        for hit in &self.hits {
            let interactive = !matches!(
                hit.action,
                HitAction::Block
                    | HitAction::ContextMenu
                    | HitAction::Move
                    | HitAction::Resize
                    | HitAction::Semantic
            );
            if interactive && (hit.rect.is_empty() || !hit.rect.is_finite()) {
                empty.push((hit.id, hit.rect));
            }
        }
        for (id, rect) in empty {
            self.report(DiagnosticKind::NoLayoutSpace, Some(id), Some(rect), || {
                "no layout space: the widget rect is empty".into()
            });
        }
    }

    /// Two painted elements share an ID. A widget collision already reported through
    /// its hit region collides in its parts too, so those add nothing.
    pub(super) fn report_paint_collision(&mut self, id: Id) {
        if !self
            .diagnostics
            .current
            .iter()
            .any(|d| d.kind == DiagnosticKind::IdCollision)
        {
            self.report(DiagnosticKind::IdCollision, Some(id), None, || {
                "duplicate id: use Ui::push_id or an explicit id_source".into()
            });
        }
    }

    pub(super) fn note_hit_collision(&mut self, hit: &HitRegion) {
        self.report(
            DiagnosticKind::IdCollision,
            Some(hit.id),
            Some(hit.rect),
            || "duplicate id: give each widget a unique id_source or push_id scope".into(),
        );
    }

    /// End of pass: collect builder reports, check scopes, publish, notify, draw.
    pub(super) fn finish_diagnostics(&mut self) {
        self.report_empty_hits();
        INVALID.with(|list| {
            for item in list.borrow_mut().drain(..) {
                let message = format!(
                    "{}: {} ({}:{})",
                    item.what,
                    item.detail,
                    item.at.file(),
                    item.at.line()
                );
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                (item.what, item.at.file(), item.at.line(), item.at.column()).hash(&mut hasher);
                self.report(
                    DiagnosticKind::InvalidValue,
                    Some(Id::new(hasher.finish())),
                    None,
                    || message,
                );
            }
        });
        if !self.scrolling.stack.is_empty()
            || !self.placements.stack.is_empty()
            || self.placements.outstanding > 0
            || self.visual_depth > 0
            || !self.visual_clips.is_empty()
        {
            self.report(DiagnosticKind::UnbalancedScope, None, None, || {
                "scope left open at the end of the pass; its closure did not return".into()
            });
            self.scrolling.stack.clear();
            self.placements.stack.clear();
            self.placements.outstanding = 0;
            self.visual_depth = 0;
            self.visual_clips.clear();
        }
        let diagnostics = &mut self.diagnostics;
        std::mem::swap(&mut diagnostics.published, &mut diagnostics.current);
        diagnostics.current.clear();
        if !diagnostics.published.is_empty() {
            if let Some(handler) = diagnostics.handler.as_mut() {
                if diagnostics.remembered.len() > MAX_REMEMBERED {
                    diagnostics.remembered.clear();
                }
                for diagnostic in &diagnostics.published {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    (diagnostic.kind, diagnostic.id, &diagnostic.message).hash(&mut hasher);
                    if diagnostics.remembered.insert(hasher.finish()) {
                        handler(diagnostic);
                    }
                }
            }
        }
        if self.diagnostics.overlay.active() {
            self.paint_debug_overlay();
        }
    }
}
