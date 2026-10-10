//! Where the interface lands in a render target the host owns.

use crate::Color;

/// A rectangle of physical pixels in a render target: origin top-left.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct PhysicalRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl PhysicalRect {
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The whole target of `size`.
    pub const fn whole([width, height]: [u32; 2]) -> Self {
        Self::new(0, 0, width, height)
    }

    pub const fn size(&self) -> [u32; 2] {
        [self.width, self.height]
    }

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Whether the rectangle lies inside a target of `size`.
    pub fn fits(&self, [width, height]: [u32; 2]) -> bool {
        self.x.checked_add(self.width).is_some_and(|x| x <= width)
            && self.y.checked_add(self.height).is_some_and(|y| y <= height)
    }
}

/// The area of the pass attachments a frame is drawn into.
///
/// `region` is where the interface goes; `target` is the full size of the attachments of the
/// host's pass, which `record` needs to hand the pass back with its default viewport and
/// scissor. The [`DrawData`](crate::DrawData) must have been built for the size of the
/// region: its logical size times its scale factor is the region in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EmbedViewport {
    pub region: PhysicalRect,
    pub target: [u32; 2],
}

impl EmbedViewport {
    /// The interface covers the whole `target`.
    pub const fn whole(target: [u32; 2]) -> Self {
        Self {
            region: PhysicalRect::whole(target),
            target,
        }
    }

    /// The interface covers `region` of a target of size `target`.
    pub const fn region(target: [u32; 2], region: PhysicalRect) -> Self {
        Self { region, target }
    }
}

/// What a render into the host's texture starts from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EmbedLoad {
    /// Draw over what is there: the host's scene is the backdrop of the interface.
    Keep,
    /// Clear the whole target first (a render pass cannot clear a sub-rectangle). The color
    /// is straight sRGB; with [`EmbedAlpha::Premultiplied`](super::EmbedAlpha) it is
    /// premultiplied by its alpha, as a transparent window does.
    Clear(Color),
}
