//! Variants and small option enums of the [`Carousel`](super::Carousel).
use crate::Vec2;

/// Presentation of the pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CarouselVariant {
    /// A card in front with the next ones peeking out below it as sheets.
    #[default]
    Stack,
    /// A wide central slide with its neighbours behind it, scaled and dimmed.
    Images,
}

/// Axis of the swipe gesture, the arrow keys and the wheel. `Images` is always horizontal.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CarouselOrientation {
    #[default]
    Horizontal,
    Vertical,
}
impl CarouselOrientation {
    pub(crate) fn axis(self) -> usize {
        match self {
            Self::Horizontal => 0,
            Self::Vertical => 1,
        }
    }
}

/// The side of the front card on which the following `Stack` sheets show.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StackDirection {
    #[default]
    Down,
    Up,
    Left,
    Right,
}
impl StackDirection {
    pub(crate) fn vector(self) -> Vec2 {
        match self {
            Self::Down => Vec2::Y,
            Self::Up => Vec2::NEG_Y,
            Self::Left => Vec2::NEG_X,
            Self::Right => Vec2::X,
        }
    }
}

/// Page indicator. Every item except `Count` and `None` is a button that goes to its page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CarouselIndicator {
    /// Small pale dots; the active page stretches into an accent pill that flows to the
    /// next page while the carousel moves.
    Pill,
    /// Equal dots; the active one is colored.
    Dots,
    /// Thin strokes; the active one is longer and brighter.
    Dashes,
    /// "3 / 7" text.
    Count,
    None,
}

/// Where the indicator sits relative to the pages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum IndicatorPosition {
    /// In its own row below the pages (right of them for a vertical carousel).
    #[default]
    After,
    /// In its own row above the pages (left of them for a vertical carousel).
    Before,
    /// Over the pages, near the trailing edge.
    Overlay,
}
