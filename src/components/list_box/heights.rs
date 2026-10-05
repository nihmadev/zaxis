//! Row geometry of a list: prefix sums over row heights, measured heights cached by key.
use super::model::{ListEntryKind, ListModel};
use crate::{components::scroll_area::RowMetrics, Id};
use std::collections::HashMap;

/// Geometry inputs; a change rebuilds the index like a new model revision does.
#[derive(Clone, Copy, PartialEq)]
pub struct Sizes {
    pub pitch: f32,
    pub header: f32,
    pub separator: f32,
    /// Item rows are measured by building them; `pitch` is their estimate until then.
    pub measured: bool,
    /// A loading row follows the last entry.
    pub footer: bool,
}

#[derive(Clone, Copy, PartialEq)]
struct Signature {
    len: usize,
    revision: u64,
    ends: [Option<Id>; 3],
    sizes: Sizes,
}

/// All-equal rows cost nothing; headers, separators or measured rows use a Fenwick tree
/// (O(log n) offset, search and update) with one `f32` height and `Id` per row.
#[derive(Default)]
pub struct HeightIndex {
    len: usize,
    sizes: Option<Sizes>,
    signature: Option<Signature>,
    uniform: bool,
    heights: Vec<f32>,
    tree: Vec<f64>,
    keys: Vec<Id>,
    measured: HashMap<Id, f32>,
    /// Indices of section headers, ascending.
    headers: Vec<usize>,
    /// Indices of entries that are not items (headers and separators), ascending.
    plain: Vec<usize>,
}

pub(super) fn footer_key() -> Id {
    Id::new("zaxis-list-footer")
}

impl HeightIndex {
    fn sizes(&self) -> Sizes {
        self.sizes.unwrap_or(Sizes {
            pitch: 1.0,
            header: 1.0,
            separator: 1.0,
            measured: false,
            footer: false,
        })
    }
    fn rows(&self) -> usize {
        self.len + usize::from(self.sizes().footer)
    }
    /// True when the entries or geometry changed and the index was rebuilt.
    pub fn sync(&mut self, model: &impl ListModel, sizes: Sizes) -> bool {
        let len = model.len();
        let at = |i: usize| (i < len).then(|| model.entry(i).key);
        let signature = Signature {
            len,
            revision: model.revision(),
            ends: [at(0), at(len / 2), at(len.wrapping_sub(1))],
            sizes,
        };
        if self.signature == Some(signature) {
            return false;
        }
        self.rebuild(model, sizes);
        self.signature = Some(signature);
        true
    }
    /// The model no longer matches what the index was built from (found while painting).
    pub fn invalidate(&mut self) {
        self.signature = None;
    }
    /// For measured lists: a row's key at `index` differs from the indexed one.
    pub fn stale(&self, index: usize, key: Id) -> bool {
        !self.uniform && self.keys.get(index).is_some_and(|k| *k != key)
    }
    fn rebuild(&mut self, model: &impl ListModel, sizes: Sizes) {
        self.len = model.len();
        self.sizes = Some(sizes);
        self.headers.clear();
        self.plain.clear();
        for i in 0..self.len {
            match model.entry(i).kind {
                ListEntryKind::Header => {
                    self.headers.push(i);
                    self.plain.push(i);
                }
                ListEntryKind::Separator => self.plain.push(i),
                ListEntryKind::Item => {}
            }
        }
        let separators = self.plain.len() > self.headers.len();
        self.uniform = !sizes.measured && !separators && sizes.header == sizes.pitch;
        self.heights.clear();
        self.tree.clear();
        self.keys.clear();
        if self.uniform {
            self.measured.clear();
            return;
        }
        // Heights of rows that left the model are dropped here, so the cache follows the model.
        let old = std::mem::take(&mut self.measured);
        for i in 0..self.len {
            let entry = model.entry(i);
            self.keys.push(entry.key);
            let measured = old.get(&entry.key).copied();
            if let Some(h) = measured {
                self.measured.insert(entry.key, h);
            }
            self.heights.push(match entry.kind {
                ListEntryKind::Item => measured.unwrap_or(sizes.pitch),
                ListEntryKind::Header => sizes.header,
                ListEntryKind::Separator => sizes.separator,
            });
        }
        if sizes.footer {
            self.keys.push(footer_key());
            self.heights.push(sizes.pitch);
        }
        let rows = self.heights.len();
        self.tree.resize(rows + 1, 0.0);
        for (i, h) in self.heights.iter().enumerate() {
            self.tree[i + 1] += f64::from(*h);
            let parent = i + 1 + ((i + 1) & (i + 1).wrapping_neg());
            if parent <= rows {
                let value = self.tree[i + 1];
                self.tree[parent] += value;
            }
        }
    }
    /// Number of cached measured heights.
    pub fn cached(&self) -> usize {
        self.measured.len()
    }
    /// Indices of the entries that are not items, ascending: what separates an entry's
    /// index from its position among the items.
    pub fn plain(&self) -> &[usize] {
        &self.plain
    }
}

impl RowMetrics for HeightIndex {
    fn count(&self) -> usize {
        self.rows()
    }
    fn total_height(&self) -> f32 {
        self.top(self.rows())
    }
    fn top(&self, index: usize) -> f32 {
        if self.uniform {
            return self.sizes().pitch * index as f32;
        }
        let (mut i, mut sum) = (index.min(self.heights.len()), 0.0);
        while i > 0 {
            sum += self.tree[i];
            i -= i & i.wrapping_neg();
        }
        sum as f32
    }
    fn height(&self, index: usize) -> f32 {
        if self.uniform {
            self.sizes().pitch
        } else {
            self.heights
                .get(index)
                .copied()
                .unwrap_or(self.sizes().pitch)
        }
    }
    fn row_at(&self, y: f32) -> usize {
        let last = self.rows().saturating_sub(1);
        if self.uniform {
            return ((y.max(0.0) / self.sizes().pitch).floor() as usize).min(last);
        }
        let (mut pos, mut rest) = (0, f64::from(y.max(0.0)));
        let mut bit = self.heights.len().checked_next_power_of_two().unwrap_or(0);
        while bit > 0 {
            if pos + bit <= self.heights.len() && self.tree[pos + bit] <= rest {
                pos += bit;
                rest -= self.tree[pos];
            }
            bit >>= 1;
        }
        pos.min(last)
    }
    fn overscan(&self) -> f32 {
        self.sizes().pitch
    }
    fn sections(&self) -> &[usize] {
        &self.headers
    }
    fn measured(&self) -> bool {
        self.sizes().measured
    }
    fn measure(&mut self, index: usize, height: f32) -> f32 {
        let Some(old) = self.heights.get(index).copied() else {
            return 0.0;
        };
        if !(height.is_finite() && height > 0.0) || (height - old).abs() < 0.01 {
            return 0.0;
        }
        let delta = height - old;
        self.heights[index] = height;
        let mut i = index + 1;
        while i < self.tree.len() {
            self.tree[i] += f64::from(delta);
            i += i & i.wrapping_neg();
        }
        self.measured.insert(self.keys[index], height);
        delta
    }
}
