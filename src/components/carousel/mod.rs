//! Paged container with two looks, [`CarouselVariant::Stack`] (cards) and
//! [`CarouselVariant::Images`] (photographs). See `Carousel` for the entry point.
//!
//! Layers are drawn through the engine's visual transforms: poses come from one signed
//! distance per layer (`pose`), the position is pointer-driven during a swipe and a spring
//! afterwards (`drive`), and input follows the same transformed geometry that is painted.

mod access;
mod api;
mod arrows;
mod drive;
mod indicator;
mod motion;
mod options;
mod photo;
mod pose;
mod region;
mod scene;
mod show;
mod slides;
mod stack;
mod state;
mod style;
mod surface;

pub use api::{Carousel, CarouselOutput, CarouselPage};
pub use options::{
    CarouselIndicator, CarouselOrientation, CarouselVariant, IndicatorPosition, StackDirection,
};
pub use state::{CarouselState, Drag};
pub use style::CarouselStyle;
