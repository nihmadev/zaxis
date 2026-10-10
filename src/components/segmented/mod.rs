//! `SegmentedControl`: a plate of equal-weight choices with a sliding raised thumb.

mod control;
#[doc(hidden)]
pub mod layout;
#[doc(hidden)]
pub mod options;
mod paint;
mod show;

pub use control::SegmentedControl;
pub use options::{
    SegmentOption, SegmentWidth, SegmentedOrientation, SegmentedSize, SegmentedVariant,
};
