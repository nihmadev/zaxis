//! Target hit testing. Geometry comes from the hits published by the last pass,
//! so layers, popups, scroll clips, parent clips and transforms apply exactly as
//! they do for ordinary input.
use super::super::{Context, HitAction};
use super::state::{Hover, TargetInfo};
use crate::components::drag_drop::DragEffect;
use crate::{Rect, Vec2};
use winit::window::CursorIcon;

impl Context {
    /// The winning target under `pointer`, or `None`.
    ///
    /// Only the topmost window or popup under the pointer takes part. Within it
    /// the most deeply nested target wins; equal depth goes to the one built
    /// later. A rejecting target ends the search unless it hands drops on to
    /// its enclosing target; if nothing accepts, the first rejection is reported.
    pub(crate) fn drag_resolve(&self, pointer: Vec2) -> Option<Hover> {
        let window = self.top_window(pointer)?;
        let mut candidates: Vec<(u16, usize, Rect, TargetInfo)> = Vec::new();
        for (index, hit) in self.interaction.previous_hits.iter().enumerate() {
            let HitAction::DropTarget { slot } = hit.action else {
                continue;
            };
            if hit.window != window || !hit.rect.contains(pointer) || !hit.clip.contains(pointer) {
                continue;
            }
            if let Some(info) = self.drag.last_targets.get(slot as usize) {
                candidates.push((info.depth, index, hit.rect, *info));
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let mut rejected = None;
        for (_, _, rect, info) in candidates {
            if info.accepts {
                return Some(hover_of(info, rect, pointer));
            }
            rejected.get_or_insert_with(|| hover_of(info, rect, pointer));
            if !info.passthrough {
                break;
            }
        }
        rejected
    }

    /// Re-resolve the hovered target for the session pointer and update the cursor.
    pub(crate) fn drag_refresh_hover(&mut self) {
        let Some(session) = &self.drag.session else {
            return;
        };
        let hover = if session.started && !session.outside {
            self.drag_resolve(session.pointer)
        } else {
            None
        };
        let session = self.drag.session.as_mut().unwrap();
        session.cursor = cursor_for(hover, session.info.effect);
        if session.hover != hover {
            session.hover = hover;
            self.dirty = true;
        }
    }
}

fn hover_of(info: TargetInfo, rect: Rect, pointer: Vec2) -> Hover {
    let local = pointer - rect.min;
    Hover {
        id: info.id,
        key: info.key,
        accepts: info.accepts,
        insertion: info
            .zones
            .filter(|_| info.accepts)
            .map(|zones| zones.classify(local, rect.size())),
        local,
        rect,
        effect: info.effect,
    }
}

pub(super) fn cursor_for(hover: Option<Hover>, effect: DragEffect) -> CursorIcon {
    match hover {
        None => CursorIcon::Grabbing,
        Some(hover) if !hover.accepts => CursorIcon::NoDrop,
        Some(hover) => match hover.effect.unwrap_or(effect) {
            DragEffect::Move => CursorIcon::Move,
            DragEffect::Copy => CursorIcon::Copy,
        },
    }
}
