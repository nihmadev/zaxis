//! Paint descriptions: what a widget asks to draw, compared between passes to reuse geometry.

use crate::{text::TextFont, Color, CornerRadius, FontWeight, Rect, Shape, Vec2};
use std::sync::Arc;

/// The widget texture of a material: an image and the part of it the shape shows.
#[derive(Clone, Debug, PartialEq)]
pub struct MaterialImage {
    pub handle: crate::ImageHandle,
    pub texture: crate::TextureId,
    /// Origin and size in 0..1 of the texture.
    pub crop: Rect,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Paint {
    Visual {
        paint: Vec<Paint>,
        transform: crate::Transform,
        opacity: f32,
    },
    Shape(Shape),
    Image {
        rect: Rect,
        uv: Rect,
        rounding: CornerRadius,
        color: Color,
        opacity: f32,
        handle: crate::ImageHandle,
        texture: crate::TextureId,
        hidden: bool,
    },
    ScrollHint {
        rect: Rect,
        axis: usize,
        color: Color,
    },
    /// The mesh of a shape drawn by a user material: local UVs over `rect` and the texture of
    /// `image` if there is one. Parameters, size and time are not part of the description;
    /// they travel with the element so that changing them never retessellates.
    Material {
        rect: Rect,
        rounding: CornerRadius,
        color: Color,
        opacity: f32,
        image: Option<MaterialImage>,
    },
    Gradient {
        rect: Rect,
        rounding: CornerRadius,
        colors: Vec<Color>,
        columns: usize,
        rows: usize,
    },
    Text {
        text: String,
        position: Vec2,
        size: f32,
        weight: FontWeight,
        wrap_width: f32,
        color: Color,
    },
    /// Text shaped with explicit options: a tab width and a family or figure style. A
    /// multi-line field paints its paragraphs with it, so painting reads the same cached
    /// layout as the field's position queries. Plain text keeps using `Text`.
    Paragraph {
        text: String,
        position: Vec2,
        size: f32,
        font: TextFont,
        wrap_width: f32,
        tab: u16,
        color: Color,
    },
    /// A paragraph whose runs differ in weight, family or color, shaped in one pass; see
    /// [`crate::text::StyleRun`]. Glyph positions come from the same cached layout as
    /// the component's hit testing.
    Rich {
        text: String,
        runs: Arc<[crate::text::StyleRun]>,
        position: Vec2,
        size: f32,
        font: TextFont,
        wrap_width: f32,
        tab: u16,
        color: Color,
    },
}

impl Paint {
    /// Text in `font`. Plain proportional text at the default tab stays `Text`, so its cache
    /// entries and translation fast paths are unchanged; anything else is a `Paragraph`.
    pub(crate) fn text(
        text: String,
        position: Vec2,
        size: f32,
        font: TextFont,
        wrap_width: f32,
        tab: u16,
        color: Color,
    ) -> Self {
        if font.is_plain() && tab == crate::text::DEFAULT_TAB {
            Self::Text {
                text,
                position,
                size,
                weight: font.weight,
                wrap_width,
                color,
            }
        } else {
            Self::Paragraph {
                text,
                position,
                size,
                font,
                wrap_width,
                tab,
                color,
            }
        }
    }

    /// The point a translation between two descriptions is measured from.
    pub(super) fn origin(&self) -> Vec2 {
        match self {
            Self::Visual {
                paint, transform, ..
            } => transform.point(paint.first().map_or(Vec2::ZERO, Paint::origin)),
            Self::Text { position, .. }
            | Self::Paragraph { position, .. }
            | Self::Rich { position, .. } => *position,
            Self::Image { rect, .. }
            | Self::Material { rect, .. }
            | Self::ScrollHint { rect, .. }
            | Self::Gradient { rect, .. } => rect.min,
            Self::Shape(shape) => match shape {
                Shape::Rect { rect, .. }
                | Shape::GradientBorder { rect, .. }
                | Shape::Gradient { rect, .. }
                | Shape::Shadow { rect, .. } => rect.min,
                Shape::Circle { center, .. } => *center,
                Shape::Line { start, .. } => *start,
            },
        }
    }

    pub(crate) fn translate(&mut self, delta: Vec2) {
        match self {
            Self::Visual {
                paint, transform, ..
            } => {
                for primitive in paint {
                    primitive.translate(delta);
                }
                transform.translation += delta - transform.vector(delta);
            }
            Self::Text { position, .. }
            | Self::Paragraph { position, .. }
            | Self::Rich { position, .. } => *position += delta,
            Self::Image { rect, .. }
            | Self::Material { rect, .. }
            | Self::ScrollHint { rect, .. }
            | Self::Gradient { rect, .. } => *rect = rect.translate(delta),
            Self::Shape(shape) => match shape {
                Shape::Rect { rect, .. }
                | Shape::GradientBorder { rect, .. }
                | Shape::Gradient { rect, .. }
                | Shape::Shadow { rect, .. } => *rect = rect.translate(delta),
                Shape::Circle { center, .. } => *center += delta,
                Shape::Line { start, end, .. } => {
                    *start += delta;
                    *end += delta;
                }
            },
        }
    }
}

/// A scroll hint is drawn by a dedicated shader path; it is always an element of its own.
pub(super) fn is_scroll_hint(paint: &[Paint]) -> bool {
    matches!(paint, [Paint::ScrollHint { .. }])
}
