//! Integer rectangles in texels, half-open: `x0..x1`, `y0..y1`.

/// A region of a texture or of the window, in texels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Area {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl Area {
    /// From a scissor rectangle `[x, y, width, height]`.
    pub fn from_scissor([x, y, w, h]: [u32; 4]) -> Self {
        Self {
            x0: x,
            y0: y,
            x1: x + w,
            y1: y + h,
        }
    }

    pub fn scissor(self) -> [u32; 4] {
        [self.x0, self.y0, self.x1 - self.x0, self.y1 - self.y0]
    }

    pub fn is_empty(self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    pub fn pixels(self) -> u64 {
        if self.is_empty() {
            0
        } else {
            u64::from(self.x1 - self.x0) * u64::from(self.y1 - self.y0)
        }
    }

    pub fn intersects(self, other: Self) -> bool {
        self.x0 < other.x1 && other.x0 < self.x1 && self.y0 < other.y1 && other.y0 < self.y1
    }

    pub fn union(self, other: Self) -> Self {
        Self {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    /// Grown by `dx` and `dy` on every side, cut to a texture of `size`.
    pub fn grow(self, dx: u32, dy: u32, [width, height]: [u32; 2]) -> Self {
        Self {
            x0: self.x0.saturating_sub(dx),
            y0: self.y0.saturating_sub(dy),
            x1: self.x1.saturating_add(dx).min(width),
            y1: self.y1.saturating_add(dy).min(height),
        }
    }

    /// The texels of level `level` that cover this region of the full-resolution image.
    pub fn to_level(self, level: u32) -> Self {
        let n = 1 << level;
        Self {
            x0: self.x0 / n,
            y0: self.y0 / n,
            x1: self.x1.div_ceil(n),
            y1: self.y1.div_ceil(n),
        }
    }

    /// The texels one level finer that a 6-tap step reads to produce this region.
    pub fn finer(self, [width, height]: [u32; 2]) -> Self {
        Self {
            x0: (self.x0 * 2).saturating_sub(2),
            y0: (self.y0 * 2).saturating_sub(2),
            x1: (self.x1 * 2 + 2).min(width),
            y1: (self.y1 * 2 + 2).min(height),
        }
    }
}
