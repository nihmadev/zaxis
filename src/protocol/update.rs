//! Partial changes of a texture payload.

/// A rectangle of texels, in physical pixels of the texture; origin top-left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TextureRegion {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl TextureRegion {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The whole texture of `size`.
    pub const fn whole([width, height]: [u32; 2]) -> Self {
        Self::new(0, 0, width, height)
    }

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Texels covered.
    pub const fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }

    /// Whether the region is non-empty and lies inside a texture of `size`.
    pub fn fits(&self, [width, height]: [u32; 2]) -> bool {
        !self.is_empty()
            && self.x.checked_add(self.width).is_some_and(|x| x <= width)
            && self.y.checked_add(self.height).is_some_and(|y| y <= height)
    }

    /// The smallest region containing both. An empty one is ignored.
    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let x0 = self.x.min(other.x);
        let y0 = self.y.min(other.y);
        let x1 = (self.x + self.width).max(other.x + other.width);
        let y1 = (self.y + self.height).max(other.y + other.height);
        Self::new(x0, y0, x1 - x0, y1 - y0)
    }

    /// Whether the regions share a texel or an edge, so that their union covers nothing more
    /// than the two together.
    pub fn touches(&self, other: &Self) -> bool {
        self.x <= other.x + other.width
            && other.x <= self.x + self.width
            && self.y <= other.y + other.height
            && other.y <= self.y + self.height
    }
}

impl TextureRegion {
    /// The regions to write for a set of changes in a texture of `size`: regions that overlap
    /// or touch are joined into their bounding box, so no texel is written twice; more than
    /// `max` of what remains become one bounding box; and a set that covers the whole
    /// texture, counting overlaps, becomes the whole texture. Regions outside the texture
    /// or empty are dropped. The producer and the backends of this crate both use this rule.
    pub fn coalesce(regions: &[Self], size: [u32; 2], max: usize) -> Vec<Self> {
        let whole = Self::whole(size);
        let mut out: Vec<Self> = Vec::with_capacity(regions.len().min(max + 1));
        for &region in regions.iter().filter(|r| r.fits(size)) {
            let mut region = region;
            while let Some(at) = out.iter().position(|r| r.touches(&region)) {
                region = region.union(out.swap_remove(at));
            }
            out.push(region);
        }
        if out.iter().map(Self::area).sum::<u64>() >= whole.area() {
            return vec![whole];
        }
        if out.len() > max.max(1) {
            return vec![out.into_iter().fold(Self::default(), Self::union)];
        }
        out
    }
}

impl From<crate::PhysicalRect> for TextureRegion {
    fn from(r: crate::PhysicalRect) -> Self {
        Self::new(r.x, r.y, r.width, r.height)
    }
}

/// What changed in a texture since an earlier revision, a hint beside the complete payload.
///
/// A backend whose copy of the texture is at exactly `base_revision`, has the same size and
/// format, and sees `to_revision` as the revision of the payload may write only `regions`
/// from the payload's pixels (rows of `size[0] * 4` bytes) and keep its texture and bindings.
/// In every other case (a lost device, a texture evicted or never uploaded, a skipped frame,
/// a replaced source) it uploads the complete payload, which is always current. Every texel
/// that differs between the two revisions must lie inside a region.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextureUpdate {
    pub id: super::TextureId,
    pub base_revision: u64,
    pub to_revision: u64,
    pub regions: Vec<TextureRegion>,
}
