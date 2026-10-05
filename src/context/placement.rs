//! Deferred content of measured cells. User code executes once; what a cell paints and
//! registers (paints, hits, scroll scopes, IME area, scroll targets) is recorded while it
//! is built, then moves and clips as a unit once the cell is measured:
//!
//! * record and accumulate (here): a placement collects the content of its window and of
//!   the popup layers opened inside it; content of scroll areas opened inside it stays in
//!   the scroll queues and is tracked by range;
//! * transform (`visual`): a visual effect wraps paints, composes transforms of hits,
//!   scales scroll scopes and blocks inert content;
//! * translate, clip and emit (`emit`): a measured offset moves everything, clips it to
//!   the placed region and hands it to the enclosing placement, the scroll queues or the
//!   frame. Emission waits until every placement and scroll scope is closed, so no
//!   partially placed content reaches the frame;
//! * portals (`portal`): the open popup of a moved cell follows its anchor.

mod emit;
mod portal;
mod visual;

use super::{Context, HitRegion, Id, Paint};
use crate::Rect;
use std::ops::Range;

pub struct PlacedPaint {
    pub id: Id,
    pub layer: Id,
    pub clip: Rect,
    pub paint: Vec<Paint>,
    pub blur: Option<f32>,
    scope: Option<usize>,
}

impl PlacedPaint {
    fn translate(&mut self, delta: crate::Vec2) {
        self.clip = self.clip.translate(delta);
        for primitive in &mut self.paint {
            primitive.translate(delta);
        }
    }
}

pub struct Placement {
    animation_start: usize,
    popup_start: usize,
    window: Id,
    pub paints: Vec<PlacedPaint>,
    pub hits: Vec<(HitRegion, Option<usize>)>,
    paints_range: Range<usize>,
    hits_range: Range<usize>,
    scopes_range: Range<usize>,
    ime: Option<(Option<usize>, Rect)>,
    targets: Vec<(usize, Rect)>,
}

impl Placement {
    /// Whether content of `layer` belongs to this placement: its window, or a popup layer
    /// opened while it was being built.
    fn collects(&self, layer: Id, popup_layers: &[Id]) -> bool {
        self.window == layer || popup_layers[self.popup_start..].contains(&layer)
    }

    /// The clip that content meets where it is placed, by its layer: the placed region in
    /// the placement's window, the viewport in a popup layer.
    fn outer_clip(&self, clip: Rect, viewport: Rect) -> impl Fn(Id) -> Rect + Copy {
        let window = self.window;
        move |layer| if layer == window { clip } else { viewport }
    }

    /// Whether content of a scroll scope (`None`: no scope) meets the placed region. A
    /// scroll area opened inside the placement clips its content at its own viewport.
    fn clipped_by(&self) -> impl Fn(Option<usize>) -> bool + Copy {
        let start = self.scopes_range.start;
        move |scope| scope.is_none_or(|scope| scope < start)
    }
}

#[derive(Default)]
pub struct Placements {
    pub stack: Vec<Placement>,
    pub outstanding: usize,
}

impl Context {
    pub(crate) fn begin_placement(&mut self, window: Id) {
        self.placements.outstanding += 1;
        self.placements.stack.push(Placement {
            animation_start: self.animations.observation(),
            popup_start: self.popups.layers.len(),
            window,
            paints: Vec::new(),
            hits: Vec::new(),
            paints_range: self.scrolling.pending.len()..0,
            hits_range: self.scrolling.hits.len()..0,
            scopes_range: self.scrolling.scopes.len()..0,
            ime: None,
            targets: Vec::new(),
        });
    }

    pub(crate) fn end_placement(&mut self) -> Placement {
        let mut p = self.placements.stack.pop().unwrap();
        p.paints_range.end = self.scrolling.pending.len();
        p.hits_range.end = self.scrolling.hits.len();
        p.scopes_range.end = self.scrolling.scopes.len();
        p
    }

    /// IDs of every region the placement holds, including those inside its scroll areas.
    pub(crate) fn placement_hit_ids(&self, p: &Placement) -> Vec<Id> {
        p.hits
            .iter()
            .map(|(h, _)| h.id)
            .chain(
                self.scrolling.hits[p.hits_range.clone()]
                    .iter()
                    .map(|(h, _)| h.id),
            )
            .collect()
    }

    pub(crate) fn hide_placement_animations(&mut self, p: &Placement) {
        self.animations.hide_since(p.animation_start);
    }

    /// The open placement that collects content of `layer`.
    fn collecting(&mut self, layer: Id) -> Option<&mut Placement> {
        let layers = &self.popups.layers;
        self.placements
            .stack
            .last_mut()
            .filter(|p| p.collects(layer, layers))
    }

    pub(crate) fn defer_placement_paint(
        &mut self,
        id: Id,
        layer: Id,
        clip: Rect,
        paint: Vec<Paint>,
    ) -> Result<(), Vec<Paint>> {
        let scope = self.scrolling.owner(layer);
        let Some(p) = self.collecting(layer) else {
            return Err(paint);
        };
        p.paints.push(PlacedPaint {
            id,
            layer,
            clip,
            paint,
            blur: None,
            scope,
        });
        Ok(())
    }

    pub(crate) fn defer_placement_hit(&mut self, hit: HitRegion) -> bool {
        let scope = self.scrolling.owner(hit.window);
        let Some(p) = self.collecting(hit.window) else {
            return false;
        };
        p.hits.push((hit, scope));
        true
    }

    pub(crate) fn defer_placement_ime(&mut self, window: Id, rect: Rect) -> bool {
        let scope = self.scrolling.owner(window);
        let Some(p) = self.collecting(window) else {
            return false;
        };
        p.ime = Some((scope, rect));
        true
    }

    /// A scroll target recorded in scroll scope `scope` of `window`, in that scope's
    /// content coordinates; placements outside the scope move it when they are placed.
    pub(crate) fn record_placement_target(&mut self, window: Id, scope: usize, target: Rect) {
        if let Some(p) = self
            .placements
            .stack
            .last_mut()
            .filter(|p| p.window == window)
        {
            p.targets.push((scope, target));
        }
    }
}
