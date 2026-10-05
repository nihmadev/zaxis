//! What the layer painters share: the frame's geometry, page content and the layer list.
use super::{state::Model, style::Look};
use crate::{ImageSource, Rect, Transform, Ui, Vec2};

/// Builds the content of a page into a UI.
pub(super) type PageFn<'a> = dyn FnMut(&mut Ui<'_>, usize) + 'a;

/// Page content of a pass. The closures live only for the pass that built them.
pub(super) enum Content<'a> {
    Pages(&'a mut PageFn<'a>),
    Photos {
        source: &'a mut dyn FnMut(usize) -> ImageSource,
        overlay: Option<&'a mut PageFn<'a>>,
    },
}

pub(super) struct Scene<'a> {
    pub(super) id: crate::Id,
    /// Clip of all layers: the pages area including the room for shadows.
    pub(super) stage: Rect,
    /// Where the front card or slide lives, inside `stage`.
    pub(super) area: Rect,
    /// Position used for layer poses; pulled inside the pages for a non-looping carousel.
    pub(super) position: f32,
    /// Distance beyond the first or last page, in pages (rubber band), else zero.
    pub(super) over: f32,
    pub(super) target: i64,
    /// Where the current movement started; equals `target` at rest.
    pub(super) from: i64,
    pub(super) model: &'a Model<'a>,
    pub(super) wrap: bool,
    pub(super) axis: usize,
    pub(super) look: &'a Look,
    pub(super) dragging: bool,
    pub(super) enabled: bool,
}

impl Scene<'_> {
    /// Page shown by the layer at unwrapped index `k`.
    pub(super) fn page(&self, k: i64) -> Option<usize> {
        let n = self.model.count as i64;
        if self.wrap {
            Some(k.rem_euclid(n) as usize)
        } else {
            (0..n).contains(&k).then_some(k as usize)
        }
    }

    /// Layers from `before` pages before the position to `after` pages past it, as
    /// `(k, page, distance)` sorted by `order`, which is the paint order. A page appears at most once.
    pub(super) fn layers(
        &self,
        before: usize,
        after: usize,
        order: fn(f32, f32) -> std::cmp::Ordering,
    ) -> Vec<(i64, usize, f32)> {
        let first = self.position.floor() as i64 - before as i64;
        let last = self.position.floor() as i64 + 1 + after as i64;
        let mut layers: Vec<_> = (first..=last)
            .filter_map(|k| Some((k, self.page(k)?, k as f32 - self.position)))
            .collect();
        if self.wrap {
            // At most one layer per page: drop the farthest repeats.
            layers.sort_by(|a, b| a.2.abs().total_cmp(&b.2.abs()));
            let mut seen = std::collections::HashSet::new();
            layers.retain(|layer| seen.insert(layer.1));
        }
        layers.sort_by(|a, b| order(a.2, b.2));
        layers
    }

    /// Whether the controls of layer `k` take input: the page we are on or heading to,
    /// close to rest, not while the pointer is dragging it.
    pub(super) fn interactive(&self, k: i64) -> bool {
        self.enabled
            && !self.dragging
            && k == self.target
            && (k as f32 - self.position).abs() < 0.15
    }

    /// Extra translation of a layer pulled past the end of a non-looping carousel.
    pub(super) fn overscroll(&self, k: i64, transform: Transform, extent: f32) -> Transform {
        let edge = if self.over < 0.0 {
            0
        } else {
            self.model.count as i64 - 1
        };
        if self.over == 0.0 || k != edge {
            return transform;
        }
        let mut shifted = transform;
        shifted.translation[self.axis] += -self.over * extent;
        shifted
    }
}

/// What a painter drew.
pub(super) struct Painted {
    /// Bounds of the drawn layers by unwrapped page, back to front.
    pub(super) layers: Vec<(i64, Rect)>,
    /// Bounds of the layer for the target page.
    pub(super) front: Rect,
}

pub(super) fn bounds(transform: Transform, base: Rect) -> Rect {
    let rect = transform.rect(base);
    if rect.min.is_finite() && rect.max.is_finite() {
        rect
    } else {
        Rect::from_min_size(base.center(), Vec2::ZERO)
    }
}
