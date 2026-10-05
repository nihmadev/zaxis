//! Look and feel of a [`Carousel`](super::Carousel).
use super::options::StackDirection;
use crate::{components::theme::SurfaceStyle, Border, Color, Padding, SpringOptions, Vec2};
use std::time::Duration;

/// Every field is optional; an unset one follows the current [`Style`](crate::Style) (the card
/// surface, accent, focus border and motion spring) or the default noted on the field.
/// Lengths are logical pixels; "pages" are carousel positions (one page is one step).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CarouselStyle {
    /// Room kept around the pages for shadows and the focus ring. Default 14.
    pub padding: Option<f32>,
    /// Distance between the pages and an indicator row outside them. Default 14.
    pub gap: Option<f32>,
    /// Corner radius of cards and slides. Default 18.
    pub rounding: Option<f32>,
    /// Patch over `Style::card.surface` for the front card.
    pub card: SurfaceStyle,
    /// Patch over the card surface for the sheets behind it.
    pub sheet: SurfaceStyle,
    /// Inset of the card content (never less than the corner arc needs). Default 16.
    pub content_padding: Option<Padding>,
    /// Stack: visible layers including the front card. Default 3.
    pub layers: Option<usize>,
    /// Stack: how far each following sheet peeks out. Default 14.
    pub offset: Option<f32>,
    /// Stack: scale lost per layer. Default 0.05.
    pub scale_step: Option<f32>,
    /// Stack: opacity lost per layer. Default 0.35.
    pub fade_step: Option<f32>,
    pub direction: Option<StackDirection>,
    /// Stack: where the card leaves to, in card sizes. Default depends on the axis.
    pub exit: Option<Vec2>,
    /// Stack: tilt of a leaving card in radians. Default 0: a tilted card is resampled
    /// every frame, which shimmers on small text; set a small angle for the effect.
    pub exit_rotation: Option<f32>,
    /// Images: neighbours visible on each side. Default 2.
    pub neighbors: Option<usize>,
    /// Images: how far each further neighbour shows past the previous one. Default 36.
    pub peek: Option<f32>,
    /// Images: scale lost per neighbour. Default 0.12.
    pub slide_scale_step: Option<f32>,
    /// Images: vertical shift per neighbour. Default 6.
    pub slide_drop: Option<f32>,
    /// Images: darkening per neighbour, 0..1. Default 0.4.
    pub dim: Option<f32>,
    /// Images: backdrop blur radius of the nearest neighbour, at most 24. Default off.
    pub blur: Option<f32>,
    /// Images: darkening under the overlay of the active slide, 0..1. Default 0.35.
    pub scrim: Option<f32>,
    /// Drag distance of one page, as a part of the page length. Default 0.5.
    pub travel: Option<f32>,
    /// Part of a page that commits a swipe without momentum. Default 0.25.
    pub commit: Option<f32>,
    /// Friction of the fling projection; larger flings less far. Default 4.
    pub friction: Option<f64>,
    /// Slope of the rubber band at a non-looping edge, 0 disables the give. Default 0.55.
    pub rubber: Option<f32>,
    pub spring: Option<SpringOptions>,
    /// Wheel distance that turns one page. Default 10.
    pub wheel_step: Option<f32>,
    /// Wheel input ignored after a turn, so trackpad inertia turns one page. Default 220 ms.
    pub wheel_lock: Option<Duration>,
    pub arrow_size: Option<f32>,
    pub arrow_inset: Option<f32>,
    /// Dot diameter and stroke thickness. Default 6.
    pub indicator_size: Option<f32>,
    pub indicator_gap: Option<f32>,
    /// Length of the active pill or stroke. Default 22.
    pub indicator_length: Option<f32>,
    pub indicator_color: Option<Color>,
    pub indicator_active: Option<Color>,
    pub focus_ring: Option<Border>,
}

macro_rules! builders {
    ($($name:ident: $ty:ty),* $(,)?) => {
        impl CarouselStyle { $(
            pub fn $name(mut self, value: $ty) -> Self { self.$name = Some(value); self }
        )* }
    };
}
builders!(
    padding: f32, gap: f32, rounding: f32, content_padding: Padding, layers: usize,
    offset: f32, scale_step: f32, fade_step: f32, direction: StackDirection, exit: Vec2,
    exit_rotation: f32, neighbors: usize, peek: f32, slide_scale_step: f32, slide_drop: f32,
    dim: f32, blur: f32, scrim: f32, travel: f32, commit: f32, friction: f64, rubber: f32,
    spring: SpringOptions, wheel_step: f32, wheel_lock: Duration, arrow_size: f32,
    arrow_inset: f32, indicator_size: f32, indicator_gap: f32, indicator_length: f32,
    indicator_color: Color, indicator_active: Color, focus_ring: Border,
);

impl CarouselStyle {
    pub fn card(mut self, surface: SurfaceStyle) -> Self {
        self.card = surface;
        self
    }
    pub fn sheet(mut self, surface: SurfaceStyle) -> Self {
        self.sheet = surface;
        self
    }
    pub(crate) fn merge(&mut self, rhs: &Self) {
        macro_rules! set { ($($f:ident),*) => { $(if rhs.$f.is_some() { self.$f = rhs.$f.clone(); })* }; }
        set!(
            padding,
            gap,
            rounding,
            content_padding,
            layers,
            offset,
            scale_step,
            fade_step,
            direction,
            exit,
            exit_rotation,
            neighbors,
            peek,
            slide_scale_step,
            slide_drop,
            dim,
            blur,
            scrim,
            travel,
            commit,
            friction,
            rubber,
            spring,
            wheel_step,
            wheel_lock,
            arrow_size,
            arrow_inset,
            indicator_size,
            indicator_gap,
            indicator_length,
            indicator_color,
            indicator_active,
            focus_ring
        );
        self.card.merge(rhs.card);
        self.sheet.merge(rhs.sheet);
    }
}

/// Finite and within `[lo, hi]`, else `default`.
fn num(value: Option<f32>, default: f32, lo: f32, hi: f32) -> f32 {
    value
        .filter(|v| v.is_finite())
        .map_or(default, |v| v.clamp(lo, hi))
}

/// Resolved values: defaults applied and every number made finite and in range.
pub(super) struct Look {
    pub(super) style: CarouselStyle,
}
impl Look {
    pub(super) fn padding(&self) -> f32 {
        num(self.style.padding, 14.0, 0.0, 200.0)
    }
    pub(super) fn gap(&self) -> f32 {
        num(self.style.gap, 14.0, 0.0, 200.0)
    }
    pub(super) fn rounding(&self) -> f32 {
        num(self.style.rounding, 18.0, 0.0, 200.0)
    }
    pub(super) fn content_padding(&self) -> Padding {
        let p = self.style.content_padding.unwrap_or(Padding::all(16.0));
        let fix = |v: f32| num(Some(v), 0.0, 0.0, 400.0);
        Padding {
            left: fix(p.left),
            right: fix(p.right),
            top: fix(p.top),
            bottom: fix(p.bottom),
        }
    }
    pub(super) fn layers(&self) -> usize {
        self.style.layers.unwrap_or(3).clamp(1, 8)
    }
    pub(super) fn offset(&self) -> f32 {
        num(self.style.offset, 14.0, 0.0, 120.0)
    }
    pub(super) fn scale_step(&self) -> f32 {
        num(self.style.scale_step, 0.05, 0.0, 0.3)
    }
    pub(super) fn fade_step(&self) -> f32 {
        num(self.style.fade_step, 0.35, 0.0, 1.0)
    }
    pub(super) fn direction(&self) -> StackDirection {
        self.style.direction.unwrap_or_default()
    }
    pub(super) fn exit(&self, axis: usize) -> Vec2 {
        self.style
            .exit
            .filter(|v| v.is_finite())
            .unwrap_or(if axis == 0 {
                Vec2::new(-1.0, -0.12)
            } else {
                Vec2::new(0.0, -1.0)
            })
    }
    pub(super) fn exit_rotation(&self) -> f32 {
        num(self.style.exit_rotation, 0.0, -1.5, 1.5)
    }
    pub(super) fn neighbors(&self) -> usize {
        self.style.neighbors.unwrap_or(2).clamp(1, 4)
    }
    pub(super) fn peek(&self) -> f32 {
        num(self.style.peek, 36.0, 0.0, 400.0)
    }
    pub(super) fn slide_scale_step(&self) -> f32 {
        num(self.style.slide_scale_step, 0.12, 0.0, 0.4)
    }
    pub(super) fn slide_drop(&self) -> f32 {
        num(self.style.slide_drop, 6.0, -200.0, 200.0)
    }
    pub(super) fn dim(&self) -> f32 {
        num(self.style.dim, 0.4, 0.0, 1.0)
    }
    pub(super) fn blur(&self) -> f32 {
        num(self.style.blur, 0.0, 0.0, 24.0)
    }
    pub(super) fn scrim(&self) -> f32 {
        num(self.style.scrim, 0.35, 0.0, 1.0)
    }
    pub(super) fn travel(&self) -> f32 {
        num(self.style.travel, 0.5, 0.05, 4.0)
    }
    pub(super) fn commit(&self) -> f32 {
        num(self.style.commit, 0.25, 0.01, 0.99)
    }
    pub(super) fn friction(&self) -> f64 {
        self.style
            .friction
            .filter(|v| v.is_finite())
            .map_or(4.0, |v| v.clamp(0.1, 100.0))
    }
    pub(super) fn rubber(&self) -> f32 {
        num(self.style.rubber, 0.55, 0.0, 4.0)
    }
    pub(super) fn wheel_step(&self) -> f32 {
        num(self.style.wheel_step, 10.0, 1.0, 1000.0)
    }
    pub(super) fn wheel_lock(&self) -> Duration {
        self.style
            .wheel_lock
            .unwrap_or(Duration::from_millis(220))
            .min(Duration::from_secs(2))
    }
    pub(super) fn arrow_size(&self) -> f32 {
        num(self.style.arrow_size, 34.0, 16.0, 120.0)
    }
    pub(super) fn arrow_inset(&self) -> f32 {
        num(self.style.arrow_inset, 10.0, 0.0, 200.0)
    }
    pub(super) fn indicator_size(&self) -> f32 {
        num(self.style.indicator_size, 6.0, 1.0, 40.0)
    }
    pub(super) fn indicator_gap(&self) -> f32 {
        num(self.style.indicator_gap, 6.0, 0.0, 80.0)
    }
    pub(super) fn indicator_length(&self) -> f32 {
        num(self.style.indicator_length, 22.0, 1.0, 200.0)
    }
}
