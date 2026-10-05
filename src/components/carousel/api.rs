//! Public builder, page binding and output of the carousel.
use super::{
    options::{
        CarouselIndicator, CarouselOrientation, CarouselVariant, IndicatorPosition, StackDirection,
    },
    style::CarouselStyle,
};
use crate::{components::sanitize, Id, Response, SpringOptions, Vec2};
use std::{hash::Hash, time::Duration};

/// The current page of a [`Carousel`]. Assigning it from outside moves the carousel with an
/// animation and reports no change; a user action writes the new page and reports one change.
pub enum CarouselPage<'a> {
    /// The page index. With [`Carousel::keys`], content state follows the keys while the
    /// index stays the source of truth: inserting a page before the current one changes
    /// which page is shown unless you adjust the index too.
    Index(&'a mut usize),
    /// The key of the page (see [`Carousel::keys`]). Inserting or removing other pages keeps
    /// this page and the motion going; removing the current page moves to its neighbour.
    Key(&'a mut Id),
    /// The carousel keeps the page itself, starting at [`Carousel::initial_page`].
    Internal,
}
impl<'a> From<&'a mut usize> for CarouselPage<'a> {
    fn from(page: &'a mut usize) -> Self {
        Self::Index(page)
    }
}
impl<'a> From<&'a mut Id> for CarouselPage<'a> {
    fn from(key: &'a mut Id) -> Self {
        Self::Key(key)
    }
}

/// What happened during one pass of a [`Carousel`].
#[derive(Clone, Copy, Debug)]
pub struct CarouselOutput {
    /// The paged area: hover, focus, `drag_*` of the swipe gesture.
    pub response: Response,
    /// Page shown or being moved to.
    pub page: usize,
    /// Key of that page.
    pub key: Id,
    /// The carousel itself changed the page this pass (swipe, key, wheel, click, button or
    /// autoplay), once per action. Assigning the page from outside never sets it.
    pub changed: bool,
    /// `changed` came from autoplay, not from the user.
    pub autoplayed: bool,
    /// Closeness of the displayed position to the current page, 0..1 and 1 when settled.
    /// It follows the pointer while dragging and dips while a transition runs.
    pub progress: f32,
    /// A swipe is in progress.
    pub dragging: bool,
    /// Enter, Space or a click activated this (front) page.
    pub activated: Option<usize>,
}

/// A paged container with two looks: a stack of cards and a photo slider.
///
/// ```no_run
/// # fn demo(ui: &mut zaxis::Ui<'_>, page: &mut usize) {
/// use zaxis::Carousel;
/// let out = Carousel::new("steps").pages(3).show(ui, page, |ui, index| {
///     ui.label(format!("Step {index}"));
/// });
/// if out.changed { /* the user moved to `*page` */ }
/// # }
/// ```
///
/// Page content closures run once per visible layer in a pass and are never stored.
#[derive(Clone)]
pub struct Carousel {
    pub(super) id: Id,
    pub(super) variant: CarouselVariant,
    pub(super) orientation: CarouselOrientation,
    pub(super) count: usize,
    pub(super) keys: Option<Vec<Id>>,
    pub(super) width: Option<f32>,
    pub(super) height: Option<f32>,
    pub(super) wrap: bool,
    pub(super) autoplay: Option<Duration>,
    pub(super) arrows: bool,
    pub(super) indicator: Option<CarouselIndicator>,
    pub(super) position: Option<IndicatorPosition>,
    pub(super) initial: usize,
    pub(super) enabled: bool,
    pub(super) wheel: bool,
    pub(super) style: CarouselStyle,
    pub(super) label: String,
}

impl Carousel {
    pub fn new(source: impl Hash) -> Self {
        Self {
            id: Id::new(source),
            variant: CarouselVariant::Stack,
            orientation: CarouselOrientation::Horizontal,
            count: 0,
            keys: None,
            width: None,
            height: None,
            wrap: false,
            autoplay: None,
            arrows: false,
            indicator: None,
            position: None,
            initial: 0,
            enabled: true,
            wheel: true,
            style: CarouselStyle::default(),
            label: String::new(),
        }
    }
    pub fn variant(mut self, variant: CarouselVariant) -> Self {
        self.variant = variant;
        self
    }
    /// A stack of cards (the default).
    pub fn stack(self) -> Self {
        self.variant(CarouselVariant::Stack)
    }
    /// A photo slider.
    pub fn images(self) -> Self {
        self.variant(CarouselVariant::Images)
    }
    pub fn orientation(mut self, orientation: CarouselOrientation) -> Self {
        self.orientation = orientation;
        self
    }
    /// Swipe, keys and wheel along the vertical axis; `Stack` only.
    pub fn vertical(self) -> Self {
        self.orientation(CarouselOrientation::Vertical)
    }
    /// Number of pages. Ignored when [`Self::keys`] is given.
    pub fn pages(mut self, count: usize) -> Self {
        self.count = count;
        self
    }
    /// Stable keys of the pages, which also set their number. Content state, focus and the
    /// motion follow a key when pages are inserted or removed. Equal keys are made unique
    /// by their index and reported.
    pub fn keys(mut self, keys: impl IntoIterator<Item = impl Hash>) -> Self {
        self.keys = Some(keys.into_iter().map(Id::new).collect());
        self
    }
    /// Outer width; the available width by default.
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = sanitize::positive("Carousel::width", width).or(self.width);
        self
    }
    /// Outer height including an indicator row outside the pages.
    #[track_caller]
    pub fn height(mut self, height: f32) -> Self {
        self.height = sanitize::positive("Carousel::height", height).or(self.height);
        self
    }
    #[track_caller]
    pub fn size(self, size: Vec2) -> Self {
        self.width(size.x).height(size.y)
    }
    /// Go from the last page to the first and back instead of stopping with a rubber band.
    pub fn looping(mut self, looping: bool) -> Self {
        self.wrap = looping;
        self
    }
    /// Turn the page every `interval` while nothing holds it: it waits while the pointer is
    /// over the carousel, while it has keyboard focus or is being dragged, while it is
    /// hidden, and never runs with reduced motion. A non-looping carousel stops at the end.
    #[track_caller]
    pub fn autoplay(mut self, interval: Duration) -> Self {
        if interval.is_zero() {
            sanitize::positive("Carousel::autoplay", 0.0);
        } else {
            self.autoplay = Some(interval);
        }
        self
    }
    /// Previous and next buttons over the pages.
    pub fn arrows(mut self, arrows: bool) -> Self {
        self.arrows = arrows;
        self
    }
    /// Default: a pill for `Stack`, strokes for `Images`.
    pub fn indicator(mut self, indicator: CarouselIndicator) -> Self {
        self.indicator = Some(indicator);
        self
    }
    pub fn indicator_position(mut self, position: IndicatorPosition) -> Self {
        self.position = Some(position);
        self
    }
    /// First page of a carousel that keeps its own page ([`CarouselPage::Internal`]).
    pub fn initial_page(mut self, page: usize) -> Self {
        self.initial = page;
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Turn pages with the wheel along the carousel axis (default on). Wheel movement
    /// across the axis goes to the enclosing scroll area.
    pub fn wheel(mut self, wheel: bool) -> Self {
        self.wheel = wheel;
        self
    }
    /// Merge `style` over the theme's [`Style::carousel`](crate::Style) and earlier calls.
    pub fn style(mut self, style: CarouselStyle) -> Self {
        self.style.merge(&style);
        self
    }
    /// `Stack`: visible layers including the front card.
    pub fn layers(mut self, layers: usize) -> Self {
        self.style.layers = Some(layers);
        self
    }
    /// `Stack`: how far each following sheet peeks out.
    #[track_caller]
    pub fn offset(mut self, offset: f32) -> Self {
        self.style.offset =
            sanitize::non_negative("Carousel::offset", offset).or(self.style.offset);
        self
    }
    /// `Stack`: scale lost per layer.
    #[track_caller]
    pub fn scale_step(mut self, step: f32) -> Self {
        self.style.scale_step =
            sanitize::non_negative("Carousel::scale_step", step).or(self.style.scale_step);
        self
    }
    /// `Stack`: the side the sheets show on.
    pub fn direction(mut self, direction: StackDirection) -> Self {
        self.style.direction = Some(direction);
        self
    }
    /// `Stack`: where a leaving card goes, in card sizes (`(-1.0, 0.0)` is one width to the left).
    #[track_caller]
    pub fn exit(mut self, exit: Vec2) -> Self {
        self.style.exit = sanitize::finite_vec2("Carousel::exit", exit).or(self.style.exit);
        self
    }
    /// `Stack`: tilt of a leaving card in radians (0 by default).
    #[track_caller]
    pub fn exit_rotation(mut self, angle: f32) -> Self {
        self.style.exit_rotation =
            sanitize::finite("Carousel::exit_rotation", angle).or(self.style.exit_rotation);
        self
    }
    /// `Images`: neighbours visible on each side.
    pub fn neighbors(mut self, neighbors: usize) -> Self {
        self.style.neighbors = Some(neighbors);
        self
    }
    /// Spring of the transition; the theme's motion spring by default.
    pub fn spring(mut self, spring: SpringOptions) -> Self {
        self.style.spring = Some(spring);
        self
    }
}
