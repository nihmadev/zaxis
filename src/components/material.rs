//! Drawing a shape with a user material.

use super::{image::fit, ImageFit, Ui};
use crate::{
    context::{MaterialImage, MaterialUse, Paint},
    Color, CornerRadius, DiagnosticKind, Id, ImageSource, MaterialId, Params, Rect, Shape,
    TextureFilter, Vec2,
};
use std::{hash::Hash, sync::Arc};

/// Default sigma of the blurred backdrop a [`reads_backdrop`](crate::Material::reads_backdrop)
/// material sees, in logical pixels.
const DEFAULT_BACKDROP_BLUR: f32 = 4.0;

/// One draw of a registered material: where, with which parameters, and over which image.
///
/// The shape is a rectangle with optional rounded corners; clip, scroll, visual transforms,
/// opacity and layer order work as for any other shape, and the mesh is cached like theirs.
/// Changing [`params`](Self::params) between frames only rewrites the draw's uniform block.
/// Hit testing is the caller's: allocate the rectangle and use [`Ui::interact`](crate::Ui::interact).
///
/// A draw that cannot use its material (not registered, invalid WGSL, parameters that do
/// not fit the schema) paints [`fallback`](Self::fallback), transparent by default, and
/// reports the reason once through [`Context::diagnostics`](crate::Context::diagnostics).
#[derive(Clone, Debug)]
pub struct MaterialPaint {
    rect: Rect,
    material: MaterialId,
    params: Params,
    rounding: CornerRadius,
    tint: Color,
    opacity: f32,
    image: Option<ImageSource>,
    fit: ImageFit,
    filter: TextureFilter,
    backdrop: f32,
    fallback: Color,
    animated: bool,
    id: Option<Id>,
}

impl MaterialPaint {
    fn new(rect: Rect, material: MaterialId) -> Self {
        Self {
            rect,
            material,
            params: Params::new(),
            rounding: CornerRadius::ZERO,
            tint: Color::WHITE,
            opacity: 1.0,
            image: None,
            fit: ImageFit::Stretch,
            filter: TextureFilter::Linear,
            backdrop: DEFAULT_BACKDROP_BLUR,
            fallback: Color::TRANSPARENT,
            animated: true,
            id: None,
        }
    }

    pub fn params(mut self, params: Params) -> Self {
        self.params = params;
        self
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = radius.into();
        self
    }
    /// The straight color the shader receives as `in.tint`. Its alpha is coverage, which the
    /// library applies after the shader.
    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = tint;
        self
    }
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = if opacity.is_finite() {
            opacity.clamp(0.0, 1.0)
        } else {
            0.0
        };
        self
    }
    /// The widget texture the shader samples with `widget_color`. The material must be
    /// declared [`reads_texture`](crate::Material::reads_texture). Pixels are those of the
    /// image once it is loaded; until then the shader sees the placeholder.
    pub fn image(mut self, source: impl Into<ImageSource>) -> Self {
        self.image = Some(source.into());
        self
    }
    /// How the image fills the shape. Stretch by default; `Cover` and `Contain` choose the
    /// part of the texture and the rectangle shown.
    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = fit;
        self
    }
    pub fn filter(mut self, filter: TextureFilter) -> Self {
        self.filter = filter;
        self
    }
    /// Sigma, in logical pixels, of the blurred backdrop of a
    /// [`reads_backdrop`](crate::Material::reads_backdrop) material; 4 by default, at most 64.
    pub fn backdrop_blur(mut self, sigma: f32) -> Self {
        self.backdrop = crate::components::blur::normalize_radius(sigma).max(0.5);
        self
    }
    /// What to paint when the material cannot be used. Transparent by default.
    pub fn fallback(mut self, color: Color) -> Self {
        self.fallback = color;
        self
    }
    /// Whether this draw reads the clock. Only meaningful for a material declared
    /// [`animated`](crate::Material::animated): `false` gives `in.time` and `in.delta` as zero
    /// and asks for no frames, for a draw that only needs to move while something else
    /// (a hover, a drag) is going on. True by default.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }
    /// A stable identity for the draw when its position among the widgets changes.
    pub fn id_source(mut self, id: impl Hash) -> Self {
        self.id = Some(Id::new(id));
        self
    }

    /// Paint the shape. Returns whether the material drew it, `false` for the fallback or
    /// an empty rectangle.
    pub fn show(self, ui: &mut Ui<'_>) -> bool {
        if !self.rect.min.is_finite() || !self.rect.max.is_finite() || self.rect.is_empty() {
            return false;
        }
        let id = match self.id {
            Some(id) => ui.scope.with(("material", id)),
            None => ui.next_id("material"),
        };
        match self.prepare(ui, id) {
            Some((paint, material, backdrop)) => {
                ui.context.paint(id, ui.window, ui.clip, vec![paint]);
                ui.context.mark_material(id, material, backdrop);
                true
            }
            None => {
                self.paint_fallback(ui, id);
                false
            }
        }
    }

    fn paint_fallback(&self, ui: &mut Ui<'_>, id: Id) {
        if self.fallback.0[3] == 0 {
            return;
        }
        let shape = Shape::rect(self.rect, self.fallback).corner_radius(self.rounding);
        ui.context
            .paint(id, ui.window, ui.clip, vec![Paint::Shape(shape.into())]);
    }

    /// The paint, the material applied to it and the backdrop sigma, or `None` after
    /// reporting why the material cannot be used.
    fn prepare(&self, ui: &mut Ui<'_>, id: Id) -> Option<(Paint, MaterialUse, Option<f32>)> {
        let program = ui.context.shared_resources().materials().get(self.material);
        let Some(program) = program else {
            ui.context
                .report(DiagnosticKind::InvalidShader, Some(id), Some(self.rect), || {
                    "the material is not registered: draw with the id Context::register_material returned"
                        .into()
                });
            return None;
        };
        if let Some(error) = &program.error {
            ui.context.report_material_error(error);
            return None;
        }
        let params = match self.params.pack(&program.layout) {
            Ok(params) => params,
            Err(error) => {
                let message = format!("{error} (material `{}`)", program.source.label);
                ui.context.report(
                    DiagnosticKind::InvalidShader,
                    Some(id),
                    Some(self.rect),
                    || message,
                );
                return None;
            }
        };
        let unit = Rect::from_min_size(Vec2::ZERO, Vec2::ONE);
        let image = self.resolve_image(ui, id, program.reads_texture);
        let (rect, crop) = match &image {
            Some((handle, _)) => {
                let intrinsic = ui
                    .context
                    .image_state(*handle)
                    .size()
                    .unwrap_or(self.rect.size());
                fit(self.rect, intrinsic, unit, self.fit)
            }
            None => (self.rect, unit),
        };
        let animated = program.animated && self.animated;
        let (time, delta) = ui.context.material_clock(animated);
        let material = MaterialUse {
            params: Arc::from(params),
            size: rect.size(),
            texture_rect: [crop.min.x, crop.min.y, crop.size().x, crop.size().y],
            time,
            delta,
            animated,
            program: Arc::clone(&program),
        };
        let paint = Paint::Material {
            rect,
            rounding: self.rounding,
            color: self.tint,
            opacity: self.opacity,
            image: image.map(|(handle, texture)| MaterialImage {
                handle,
                texture,
                crop,
            }),
        };
        let backdrop = program.reads_backdrop.then_some(self.backdrop);
        Some((paint, material, backdrop))
    }

    /// The image of this draw if the material reads one and it can be resolved.
    fn resolve_image(
        &self,
        ui: &mut Ui<'_>,
        id: Id,
        reads_texture: bool,
    ) -> Option<(crate::ImageHandle, crate::TextureId)> {
        let source = self.image.clone()?;
        if !reads_texture {
            ui.context.report(
                DiagnosticKind::InvalidUsage,
                Some(id),
                Some(self.rect),
                || "the material does not declare reads_texture, so the image is ignored".into(),
            );
            return None;
        }
        let handle = ui.context.images.lock().resolve(source);
        match handle {
            Ok(handle) => {
                let texture = ui.context.images.lock().texture(handle, self.filter)?;
                Some((handle, texture))
            }
            Err(error) => {
                ui.context
                    .report(DiagnosticKind::External, Some(id), Some(self.rect), || {
                        format!("material image failed: {error}")
                    });
                None
            }
        }
    }
}

impl Ui<'_> {
    /// Start a draw of `material` over `rect`; finish it with [`MaterialPaint::show`].
    ///
    /// ```
    /// # use zaxis::{vec2, Context, Material, Rect, Window};
    /// # let mut context = Context::new();
    /// let flat = context.register_material(&Material::new("flat", r#"
    ///     fn material(in: MaterialInput, p: Params) -> vec4<f32> {
    ///         return premultiply(vec4<f32>(in.uv, 0.5, 1.0));
    ///     }
    /// "#));
    /// # context.run(|context| { Window::new("w").show(context, |ui| {
    /// ui.material(Rect::from_min_size(vec2(8.0, 8.0), vec2(64.0, 64.0)), flat).show(ui);
    /// # }); });
    /// ```
    pub fn material(&self, rect: Rect, material: MaterialId) -> MaterialPaint {
        MaterialPaint::new(rect, material)
    }
}
