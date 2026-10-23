//! The host's side of a live frame: a model of the pixels, the region each step changes and
//! what the target must show for each layout.
use zaxis::{ImageSource, TextureRegion};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Rgba,
    Bgra,
    BgraOpaque,
}

impl Layout {
    pub fn name(self) -> &'static str {
        match self {
            Self::Rgba => "rgba",
            Self::Bgra => "bgra",
            Self::BgraOpaque => "bgra_opaque",
        }
    }
    pub fn source(self, size: [u32; 2], bytes: impl Into<std::sync::Arc<Vec<u8>>>) -> ImageSource {
        match self {
            Self::Rgba => ImageSource::rgba(size, bytes),
            Self::Bgra => ImageSource::bgra(size, bytes),
            Self::BgraOpaque => ImageSource::bgra_opaque(size, bytes),
        }
    }
}

/// How much of the frame a step changes.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A band of 64 rows across the frame, as a scroll exposes.
    Rows64,
    /// A quarter of the frame.
    Quarter,
    Whole,
}

impl Shape {
    pub const ALL: [Self; 3] = [Self::Rows64, Self::Quarter, Self::Whole];
    pub fn name(self) -> &'static str {
        match self {
            Self::Rows64 => "rows64",
            Self::Quarter => "quarter",
            Self::Whole => "whole",
        }
    }
}

pub struct Model {
    pub size: [u32; 2],
    pub bytes: Vec<u8>,
}

impl Model {
    pub fn new(size: [u32; 2]) -> Self {
        let bytes = (0..size[0] as usize * size[1] as usize * 4)
            .map(|i| {
                if i % 4 == 3 {
                    255
                } else {
                    (i.wrapping_mul(31) >> 3) as u8
                }
            })
            .collect();
        Self { size, bytes }
    }

    pub fn region(&self, shape: Shape, step: usize) -> TextureRegion {
        let [w, h] = self.size;
        match shape {
            Shape::Rows64 => {
                TextureRegion::new(0, ((step as u32 * 64) % (h - 64)).min(h - 64), w, 64)
            }
            Shape::Quarter => TextureRegion::new(
                0,
                if step.is_multiple_of(2) { 0 } else { h / 2 },
                w / 2,
                h / 2,
            ),
            Shape::Whole => TextureRegion::whole(self.size),
        }
    }

    /// New pixels for `region` of step `step`, tightly packed, already in the model.
    pub fn paint(&mut self, region: TextureRegion, step: usize) -> Vec<u8> {
        let row = region.width as usize * 4;
        let mut patch = Vec::with_capacity(row * region.height as usize);
        for y in 0..region.height as usize {
            patch.extend((0..row).map(|i| {
                if i % 4 == 3 {
                    255
                } else {
                    (i + y * 7 + step * 53) as u8
                }
            }));
        }
        let stride = self.size[0] as usize * 4;
        for y in 0..region.height as usize {
            let to = (region.y as usize + y) * stride + region.x as usize * 4;
            self.bytes[to..to + row].copy_from_slice(&patch[y * row..][..row]);
        }
        patch
    }

    /// The sRGB pixels a 1:1 target must show for the model in `layout`.
    pub fn expected(&self, layout: Layout) -> Vec<[u8; 4]> {
        self.bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| match layout {
                Layout::Rgba => *p,
                Layout::Bgra => [p[2], p[1], p[0], p[3]],
                Layout::BgraOpaque => [p[2], p[1], p[0], 255],
            })
            .collect()
    }
}
