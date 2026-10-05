//! Retained state of a carousel and its reconciliation with the page model.
use super::{api::CarouselPage, motion};
use crate::time::Instant;
use crate::{Id, Rect};

/// A swipe in progress. `raw` is the pointer-driven position in pages before the rubber band.
#[derive(Clone, Copy, Debug)]
pub struct Drag {
    pub raw: f32,
    pub start: i64,
}

/// Retained per carousel and dropped after the first pass that does not show it.
#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct CarouselState {
    pub last_frame: u64,
    pub seen: bool,
    pub count: usize,
    /// Committed page in an unwrapped coordinate: a looping carousel counts on past the ends.
    pub target: i64,
    /// Page the current movement started from; equals `target` at rest.
    pub from: i64,
    pub page: usize,
    pub key: Option<Id>,
    /// Position displayed by the previous pass, in the same coordinate.
    pub position: f32,
    pub drag: Option<Drag>,
    /// The previous pass ended at rest.
    pub settled: bool,
    pub velocity: motion::Velocity,
    pub wheel: f32,
    pub wheel_until: Option<Instant>,
    pub autoplay_at: Option<Instant>,
    /// Bounds of the drawn layers by unwrapped page, back to front, from the previous pass.
    pub layers: Vec<(i64, Rect)>,
}

/// The pages of a pass: their number and keys.
pub(super) struct Model<'a> {
    pub(super) count: usize,
    pub(super) keys: Option<&'a [Id]>,
}
impl Model<'_> {
    pub(super) fn key(&self, index: usize) -> Id {
        self.keys
            .and_then(|keys| keys.get(index).copied())
            .unwrap_or_else(|| Id::new(index))
    }
    fn index_of(&self, key: Id) -> Option<usize> {
        (0..self.count).find(|&i| self.key(i) == key)
    }
}

/// Result of [`CarouselState::sync`].
pub(super) struct Synced {
    /// The model assigned a new page: animate to it, report nothing.
    pub(super) external: bool,
    /// Pages the displayed position moves with the rebased target, to keep it where it is.
    pub(super) shift: f32,
}

impl CarouselState {
    pub(super) fn page_of(&self, target: i64, wrap: bool) -> usize {
        let n = self.count.max(1) as i64;
        if wrap {
            target.rem_euclid(n) as usize
        } else {
            target.clamp(0, n - 1) as usize
        }
    }

    /// Reconcile with the model at the start of a pass: first sight, inserted and removed
    /// pages, and a page assigned from outside.
    pub(super) fn sync(
        &mut self,
        model: &Model<'_>,
        page: &CarouselPage<'_>,
        wrap: bool,
        initial: usize,
    ) -> Synced {
        let n = model.count;
        let last = n - 1;
        let requested = match page {
            CarouselPage::Index(i) => Some((**i).min(last)),
            CarouselPage::Key(key) => model.index_of(**key),
            CarouselPage::Internal => None,
        };
        if !self.seen {
            let first = requested.unwrap_or(initial.min(last));
            *self = Self {
                seen: true,
                count: n,
                target: first as i64,
                from: first as i64,
                page: first,
                key: Some(model.key(first)),
                position: first as f32,
                last_frame: self.last_frame,
                ..Self::default()
            };
            return Synced {
                external: false,
                shift: 0.0,
            };
        }
        // Where the page we show now lives in the new model.
        let follows_key = !matches!(page, CarouselPage::Index(_)) && model.keys.is_some();
        let here = if follows_key {
            self.key
                .and_then(|key| model.index_of(key))
                .unwrap_or(self.page.min(last))
        } else {
            self.page.min(last)
        };
        let mut shift = 0.0;
        if n != self.count || here != self.page {
            let rebased = here as i64;
            shift = (rebased - self.target) as f32;
            self.target = rebased;
            self.from = rebased;
            self.count = n;
        }
        let mut external = false;
        let moved_key = matches!(page, CarouselPage::Key(key) if Some(**key) != self.key);
        let moved_index = matches!(page, CarouselPage::Index(_));
        if let Some(wanted) = requested.filter(|&w| w != here && (moved_key || moved_index)) {
            self.target += motion::distance(here, wanted, n, wrap);
            external = true;
            self.drag = None;
        }
        self.page = self.page_of(self.target, wrap);
        self.key = Some(model.key(self.page));
        Synced { external, shift }
    }

    /// Move to `target`; returns whether the page changed.
    pub(super) fn go(&mut self, target: i64, model: &Model<'_>, wrap: bool) -> bool {
        let target = if wrap {
            target
        } else {
            target.clamp(0, model.count as i64 - 1)
        };
        if target == self.target {
            return false;
        }
        self.from = self.position.round() as i64;
        self.target = target;
        let page = self.page_of(target, wrap);
        let changed = page != self.page;
        self.page = page;
        self.key = Some(model.key(page));
        changed
    }
}
