//! The open popup of placed content: it is a portal in its own layer, anchored to a widget
//! inside the cell. When the cell moves, the anchor moves with it and the popup is fitted
//! to the viewport again on its own, independently of the cell's clip; when the cell is
//! transformed, anchor and popup are transformed with it.

use super::Placement;
use crate::{
    context::{Context, Id},
    Transform, Vec2,
};

/// The open popup of a moved placement and where its content goes.
#[derive(Clone, Copy)]
pub(super) struct Portal {
    layer: Id,
    /// How far the popup moved after fitting the viewport again.
    shift: Vec2,
    /// The trigger proxy in the popup layer, which follows the anchor.
    key_target: Option<Id>,
}

impl Portal {
    /// How far content of `window` moves: with the portal in its layer, with the
    /// placement otherwise.
    pub(super) fn shift(portal: Option<Portal>, window: Id, delta: Vec2) -> Vec2 {
        portal
            .filter(|portal| portal.layer == window)
            .map_or(delta, |portal| portal.shift)
    }

    /// Whether content of `window` moves at all: the placement's own window, or the portal.
    pub(super) fn moves(portal: Option<Portal>, p: &Placement, window: Id) -> bool {
        window == p.window || portal.is_some_and(|portal| portal.layer == window)
    }

    /// How far a hit of `window` moves. In the portal layer the full-viewport blocker
    /// stays, the trigger proxy follows the anchor and the panel's regions move with it.
    pub(super) fn hit_shift(portal: Option<Portal>, window: Id, hit: Id, delta: Vec2) -> Vec2 {
        match portal.filter(|portal| portal.layer == window) {
            Some(portal) if hit == crate::components::popup::block_id(portal.layer) => Vec2::ZERO,
            Some(portal) if Some(hit) == portal.key_target => delta,
            Some(portal) => portal.shift,
            None => delta,
        }
    }
}

impl Context {
    /// Move the open popup of `p` with its anchor by `delta`, keeping its gap to the
    /// anchor, and fit it to the viewport again. `None` when the placement does not move
    /// or opened no popup.
    pub(super) fn move_portal(&mut self, p: &Placement, delta: Vec2) -> Option<Portal> {
        if delta == Vec2::ZERO {
            return None;
        }
        let viewport = self.viewport();
        let popup = self
            .popups
            .current
            .as_mut()
            .filter(|popup| p.paints.iter().any(|paint| paint.layer == popup.id))?;
        let old = popup.rect;
        let gap = if old.min.y >= popup.anchor.max.y {
            old.min.y - popup.anchor.max.y
        } else {
            popup.anchor.min.y - old.max.y
        };
        popup.anchor = popup.anchor.translate(delta);
        popup.rect =
            crate::components::popup::place(popup.anchor, old.size(), viewport, gap.max(0.0)).0;
        Some(Portal {
            layer: popup.id,
            shift: popup.rect.min - old.min,
            key_target: popup.key_target,
        })
    }

    /// Transform the anchor and panel of the open popup of `p` with its content.
    pub(super) fn transform_portal(&mut self, p: &Placement, transform: Transform) {
        if let Some(popup) = self
            .popups
            .current
            .as_mut()
            .filter(|popup| p.paints.iter().any(|paint| paint.layer == popup.id))
        {
            popup.anchor = transform.rect(popup.anchor);
            popup.rect = transform.rect(popup.rect);
        }
    }
}
