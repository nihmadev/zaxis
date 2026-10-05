use super::{Response, Ui, Widget};
use crate::{
    context::{HitAction, HitRegion, Paint},
    Color, CornerRadius, Id, ImageSource, ImageState, Rect, TextureFilter, Vec2,
};
use std::hash::Hash;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageFit {
    #[default]
    Contain,
    Cover,
    Stretch,
}

/// Local image component. Layout uses logical pixels; raster resolution uses DPI
/// and enclosing visual scale. The default preserves aspect.
///
/// Screen readers announce an image by its [`Image::alt`] text. One that only decorates
/// is marked [`Image::decorative`] and skipped; an image with neither is reported as
/// [`DiagnosticKind::MissingAccessibleName`](crate::DiagnosticKind) while assistive
/// technology is connected.
///
/// ```no_run
/// # fn images(ui: &mut zaxis::Ui<'_>) {
/// use zaxis::{Image, ImageFit, ImageSource, TextureFilter, vec2};
/// ui.image("assets/photo.jpg");
/// ui.add(Image::new("assets/photo.jpg").alt("Harbour at dusk"));
/// ui.add(Image::new("assets/icon.svg").size(vec2(96.0, 96.0)).corner_radius(12.0).decorative());
/// let pixels = ImageSource::rgba([2, 2], vec![255; 16]);
/// ui.add(Image::new(&pixels).size(vec2(64.0,64.0)).filter(TextureFilter::Nearest).fit(ImageFit::Cover));
/// # }
/// ```
pub struct Image {
    source: ImageSource,
    id: Option<Id>,
    size: Option<Vec2>,
    max_size: Vec2,
    aspect: Option<f32>,
    fit: ImageFit,
    tint: Color,
    opacity: f32,
    rounding: CornerRadius,
    uv: Rect,
    filter: TextureFilter,
    interactive: bool,
    placeholder: bool,
    alt: Option<String>,
    decorative: bool,
}
#[derive(Clone, Debug)]
pub struct ImageOutput {
    pub response: Response,
    pub state: ImageState,
}

impl Image {
    pub fn new(source: impl Into<ImageSource>) -> Self {
        Self {
            source: source.into(),
            id: None,
            size: None,
            max_size: Vec2::splat(f32::INFINITY),
            aspect: None,
            fit: ImageFit::Contain,
            tint: Color::WHITE,
            opacity: 1.0,
            rounding: CornerRadius::ZERO,
            uv: Rect::from_min_size(Vec2::ZERO, Vec2::ONE),
            filter: TextureFilter::Linear,
            interactive: false,
            placeholder: true,
            alt: None,
            decorative: false,
        }
    }
    /// What the image shows, in words: the name screen readers announce. An empty text
    /// marks the image [`Self::decorative`].
    pub fn alt(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.decorative = text.is_empty();
        self.alt = (!text.is_empty()).then_some(text);
        self
    }
    /// The image adds nothing the text around it does not say: assistive technology skips
    /// it. An [`Self::interactive`] image is a control and stays in the tree, so it still
    /// needs [`Self::alt`].
    pub fn decorative(mut self) -> Self {
        self.decorative = true;
        self.alt = None;
        self
    }
    pub fn id_source(mut self, id: impl Hash) -> Self {
        self.id = Some(Id::new(id));
        self
    }
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = Some(size);
        self
    }
    pub fn max_size(mut self, size: Vec2) -> Self {
        self.max_size = size;
        self
    }
    /// Reserves space before intrinsic metadata arrives (width / height).
    pub fn aspect_ratio(mut self, ratio: f32) -> Self {
        self.aspect = Some(ratio);
        self
    }
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = radius.into();
        self
    }
    /// Normalized source crop, clipped to `[0, 1]`. Cover applies a further centered crop.
    pub fn uv(mut self, uv: Rect) -> Self {
        self.uv = uv;
        self
    }
    pub fn filter(mut self, filter: TextureFilter) -> Self {
        self.filter = filter;
        self
    }
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
    /// Suppress the default placeholder to paint application-specific loading/errors.
    pub fn placeholder(mut self, placeholder: bool) -> Self {
        self.placeholder = placeholder;
        self
    }
    pub fn show(mut self, ui: &mut Ui<'_>) -> ImageOutput {
        let handle = ui.context.images.lock().resolve(self.source.clone());
        let state = match &handle {
            Ok(h) => ui.context.image_state(*h),
            Err(e) => ImageState::Error(e.clone()),
        };
        if let Ok(h) = handle {
            self.source = h.into();
        }
        let response = ui.add(self);
        ImageOutput { response, state }
    }
}
impl Widget for Image {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let id = match self.id {
            Some(id) => ui.scope.with(("image", id)),
            None => ui.next_id("image"),
        };
        let handle = ui.context.images.lock().resolve(self.source);
        let state = match &handle {
            Ok(h) => ui.context.image_state(*h),
            Err(e) => ImageState::Error(e.clone()),
        };
        let uv = if self.uv.min.is_finite() && self.uv.max.is_finite() {
            self.uv
                .intersect(Rect::from_min_size(Vec2::ZERO, Vec2::ONE))
        } else {
            Rect::from_min_size(Vec2::ZERO, Vec2::ONE)
        };
        let intrinsic = state.size().unwrap_or_else(|| {
            let ratio = self
                .aspect
                .filter(|n| n.is_finite() && *n > 0.0)
                .unwrap_or(1.0);
            Vec2::new(64.0, 64.0 / ratio)
        }) * uv.size();
        let desired = self
            .size
            .filter(|s| s.is_finite())
            .unwrap_or(intrinsic)
            .max(Vec2::ZERO);
        let limit = self
            .max_size
            .max(Vec2::ZERO)
            .min(Vec2::new(ui.available_width(), ui.available_height()));
        let size = if self.fit == ImageFit::Stretch {
            desired.min(limit)
        } else {
            desired
                * (limit / desired.max(Vec2::splat(0.001)))
                    .min_element()
                    .min(1.0)
                    .clamp(0.0, 1.0)
        };
        let rect = ui.allocate_space(size);
        if let ImageState::Error(error) = &state {
            ui.context.report(
                crate::DiagnosticKind::External,
                Some(id),
                Some(rect),
                || format!("image failed: {error}"),
            );
        }
        let response = ui.response(id, rect, self.interactive);
        if self.interactive {
            ui.context.register_hit(HitRegion {
                id,
                window: ui.window,
                rect,
                clip: ui.clip,
                action: if ui.is_enabled() {
                    HitAction::Activate
                } else {
                    HitAction::Block
                },
            });
        }
        if !self.decorative || self.interactive {
            ui.a11y(id, rect, crate::AccessRole::Image, |node| {
                node.busy(matches!(state, ImageState::Loading { .. }));
                if let Some(alt) = &self.alt {
                    node.label(alt.as_str());
                }
                if let ImageState::Error(error) = &state {
                    node.description(format!("image failed: {error}"));
                }
                if self.interactive {
                    node.clicks(id);
                }
            });
        }
        if let Ok(handle) = handle {
            let (paint_rect, uv) = fit(rect, intrinsic, uv, self.fit);
            let texture = ui
                .context
                .images
                .lock()
                .texture(handle, self.filter)
                .unwrap();
            // Always carry demand through placement/scroll/visual materialization.
            // Invisible sources don't enqueue jobs; suppressed placeholders still load.
            ui.context.paint(
                id,
                ui.window,
                ui.clip,
                vec![Paint::Image {
                    rect: paint_rect,
                    uv,
                    rounding: self.rounding,
                    color: self.tint,
                    opacity: if self.opacity.is_finite() {
                        self.opacity.clamp(0.0, 1.0)
                    } else {
                        0.0
                    },
                    handle,
                    texture,
                    hidden: !self.placeholder && !state.is_ready(),
                }],
            );
        }
        response
    }
}
impl Ui<'_> {
    /// Paint `source` contained in `rect` without allocating layout space, for
    /// components that draw an icon inside their own geometry.
    pub(super) fn paint_image_in(&mut self, id: Id, source: ImageSource, rect: Rect, tint: Color) {
        let handle = self.context.images.lock().resolve(source);
        let Ok(handle) = handle else { return };
        let state = self.context.image_state(handle);
        let intrinsic = state.size().unwrap_or(rect.size());
        let full = Rect::from_min_size(Vec2::ZERO, Vec2::ONE);
        let (rect, uv) = fit(rect, intrinsic, full, ImageFit::Contain);
        let texture = self
            .context
            .images
            .lock()
            .texture(handle, TextureFilter::Linear)
            .unwrap();
        self.context.paint(
            id,
            self.window,
            self.clip,
            vec![Paint::Image {
                rect,
                uv,
                rounding: CornerRadius::ZERO,
                color: tint,
                opacity: 1.0,
                handle,
                texture,
                hidden: false,
            }],
        );
    }

    pub fn image(&mut self, source: impl Into<ImageSource>) -> Response {
        self.add(Image::new(source))
    }
}
pub fn fit(rect: Rect, intrinsic: Vec2, uv: Rect, mode: ImageFit) -> (Rect, Rect) {
    if rect.is_empty() || intrinsic.min_element() <= 0.0 {
        return (rect, uv);
    }
    match mode {
        ImageFit::Stretch => (rect, uv),
        ImageFit::Contain => {
            let size = intrinsic * (rect.size() / intrinsic).min_element();
            (Rect::from_min_size(rect.center() - size * 0.5, size), uv)
        }
        ImageFit::Cover => {
            let visible = rect.size() / (intrinsic * (rect.size() / intrinsic).max_element());
            let size = uv.size() * visible;
            (rect, Rect::from_min_size(uv.center() - size * 0.5, size))
        }
    }
}
