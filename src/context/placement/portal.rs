//! The open popups of placed content: each is a portal in its own layer, anchored to a
//! widget inside the cell (or inside the popup it was opened from). When the cell moves,
//! the anchors move with it and every popup is fitted to the viewport again on its own,
//! independently of the cell's clip: a child fits after its parent moved. When the cell
//! is transformed, anchors and panels are transformed with it. A popup's extra panels
//! (cascading menus) move with it.

use super::Placement;
use crate::{
    components::popup::{block_id, place},
    context::{Context, Id},
    Transform, Vec2,
};

/// One open popup of a moved placement and where its content goes.
struct Portal {
    layer: Id,
    /// Its extra panels, which move with it.
    extra: Vec<Id>,
    /// How far the popup moved after fitting the viewport again.
    shift: Vec2,
    /// How far its trigger proxy moved: with the cell, or with the popup it sits in.
    anchor_delta: Vec2,
    /// The trigger proxy in the popup layer, which follows the anchor.
    key_target: Option<Id>,
}

/// The open popups of a moved placement, root first.
#[derive(Default)]
pub(super) struct Portals(Vec<Portal>);

impl Portals {
    fn of(&self, window: Id) -> Option<&Portal> {
        self.0
            .iter()
            .find(|p| p.layer == window || p.extra.contains(&window))
    }

    /// How far content of `window` moves: with its portal in its layer, with the
    /// placement otherwise.
    pub(super) fn shift(&self, window: Id, delta: Vec2) -> Vec2 {
        self.of(window).map_or(delta, |portal| portal.shift)
    }

    /// Whether content of `window` moves at all: the placement's own window, or a portal.
    pub(super) fn moves(&self, p: &Placement, window: Id) -> bool {
        window == p.window || self.of(window).is_some()
    }

    /// How far a hit of `window` moves. In a portal layer the full-viewport blocker
    /// stays, the trigger proxy follows its anchor and the panel's regions move with it.
    pub(super) fn hit_shift(&self, window: Id, hit: Id, delta: Vec2) -> Vec2 {
        match self.of(window) {
            Some(portal) if window == portal.layer && hit == block_id(portal.layer) => Vec2::ZERO,
            Some(portal) if Some(hit) == portal.key_target => portal.anchor_delta,
            Some(portal) => portal.shift,
            None => delta,
        }
    }
}

impl Context {
    /// Move the open popups of `p` with their anchors by `delta`, keeping each gap to its
    /// anchor, and fit each to the viewport again, parents first. Empty when the
    /// placement does not move or opened no popup.
    pub(super) fn move_portals(&mut self, p: &Placement, delta: Vec2) -> Portals {
        let mut portals = Portals::default();
        if delta == Vec2::ZERO {
            return portals;
        }
        let viewport = self.viewport();
        for popup in &mut self.popups.branch {
            if !p.paints.iter().any(|paint| paint.layer == popup.id) {
                continue;
            }
            // A trigger inside a popup that moved follows that popup, not the cell.
            let anchor_delta = popup
                .parent
                .and_then(|parent| portals.0.iter().find(|portal| portal.layer == parent))
                .map_or(delta, |parent| parent.shift);
            let old = popup.rect;
            let gap = if old.min.y >= popup.anchor.max.y {
                old.min.y - popup.anchor.max.y
            } else {
                popup.anchor.min.y - old.max.y
            };
            popup.anchor = popup.anchor.translate(anchor_delta);
            popup.rect = place(popup.anchor, old.size(), viewport, gap.max(0.0)).0;
            let shift = popup.rect.min - old.min;
            for (_, rect) in &mut popup.extra {
                *rect = rect.translate(shift);
            }
            portals.0.push(Portal {
                layer: popup.id,
                extra: popup.extra.iter().map(|(layer, _)| *layer).collect(),
                shift,
                anchor_delta,
                key_target: popup.key_target,
            });
        }
        portals
    }

    /// Transform the anchors and panels of the open popups of `p` with their content.
    pub(super) fn transform_portals(
        &mut self,
        p: &Placement,
        transform: Transform,
    ) -> VisualPortals {
        let mut portals = VisualPortals::default();
        let viewport = self.viewport();
        for popup in &mut self.popups.branch {
            if !p.paints.iter().any(|paint| paint.layer == popup.id) {
                continue;
            }
            // What the popup's own content was mapped by before fitting: the scene, or
            // the panel of the popup it was opened from.
            let base = popup
                .parent
                .and_then(|parent| portals.0.iter().find(|portal| portal.layer == parent))
                .map_or(transform, |parent| parent.panel);
            let old = popup.rect;
            let gap = (old.min.y - popup.anchor.max.y)
                .max(popup.anchor.min.y - old.max.y)
                .max(0.0);
            popup.anchor = base.rect(popup.anchor);
            let mapped = base.rect(old);
            popup.rect = place(popup.anchor, mapped.size(), viewport, gap * base.scale).0;
            let mut panel = base;
            panel.translation += popup.rect.min - mapped.min;
            for (_, rect) in &mut popup.extra {
                *rect = panel.rect(*rect);
            }
            portals.0.push(VisualPortal {
                layer: popup.id,
                extra: popup.extra.iter().map(|(layer, _)| *layer).collect(),
                key_target: popup.key_target,
                anchor: base,
                panel,
            });
        }
        portals
    }
}

/// A visual portal's panel fits the screen independently; its anchor follows the scene.
struct VisualPortal {
    layer: Id,
    extra: Vec<Id>,
    key_target: Option<Id>,
    /// Maps the trigger proxy.
    anchor: Transform,
    panel: Transform,
}

#[derive(Default)]
pub(super) struct VisualPortals(Vec<VisualPortal>);

impl VisualPortals {
    fn of(&self, layer: Id) -> Option<&VisualPortal> {
        self.0
            .iter()
            .find(|p| p.layer == layer || p.extra.contains(&layer))
    }

    pub(super) fn moves(&self, layer: Id, window: Id) -> bool {
        layer == window || self.of(layer).is_some()
    }

    pub(super) fn mapping(&self, layer: Id, id: Id, scene: Transform) -> Transform {
        match self.of(layer) {
            Some(p) if layer == p.layer && id == block_id(p.layer) => Transform::IDENTITY,
            Some(p) if layer == p.layer && Some(id) == p.key_target => p.anchor,
            Some(p) => p.panel,
            None => scene,
        }
    }
}
