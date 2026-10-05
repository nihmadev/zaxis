//! The disclosure row: hover surface, then (after the editor ran, so the preview shows the
//! final color of the pass) the swatch, caption, chevron and focus ring.

use super::{rounded, visible_label, Paint, Shape};
use crate::{
    components::appearance::Appearance, Border, Color, CornerRadius, HoverStyle, Id, Rect,
    Response, Ui, Vec2,
};

/// Geometry of the row of one picker.
#[derive(Clone, Copy)]
pub(super) struct Row {
    pub rect: Rect,
    /// Content is inset from the hover surface by this much.
    inset: f32,
    pub swatch: Rect,
}

impl Row {
    /// The chevron leads the label; the color swatch is the only trailing element.
    pub(super) fn new(rect: Rect, width: f32) -> Self {
        let inset = 8.0_f32.min(width * 0.25);
        let size = Vec2::new((width - 2.0 * inset).clamp(0.0, 28.0), 16.0);
        let swatch = Rect::from_min_size(
            Vec2::new(
                (rect.max.x - inset - size.x).max(rect.min.x),
                rect.center().y - size.y * 0.5,
            ),
            size,
        );
        Self {
            rect,
            inset,
            swatch,
        }
    }
}

/// Paint the row's hover surface; the returned appearance colors the row's text.
pub(super) fn surface(
    ui: &mut Ui<'_>,
    id: Id,
    row: Row,
    mut response: Response,
    (style, hover): (&crate::Style, Option<HoverStyle>),
) -> Appearance {
    let component = style.color_picker;
    let preset = hover.or(ui.hover_style).unwrap_or(style.hover_style);
    response.rect = row.rect;
    let mut base = Appearance::new(Color::TRANSPARENT, Border::NONE, style.text_color);
    base.rounding = component.rounding.unwrap_or(CornerRadius::all(4.0));
    base.blur = 0.0;
    base.opacity = style.opacity;
    let appearance = ui.animate_control(
        response,
        preset,
        hover.or(ui.hover_style).is_some(),
        component.body,
        crate::ControlState::from_response(response, false),
        base,
        style.button_hovered,
    );
    let mut paint = Vec::new();
    appearance.paint_shadow(row.rect, appearance.rounding, &mut paint);
    appearance.paint_body(
        row.rect,
        appearance.rounding,
        style,
        appearance.blur,
        &mut paint,
    );
    let (_, filter) = ui.resolved_blur(appearance.blur, component.body.has_blur_override());
    ui.context.paint_blur(
        id.with("row-blur"),
        ui.window,
        ui.clip,
        crate::Blur::new(row.rect)
            .radius(filter)
            .corner_radius(appearance.rounding),
    );
    ui.context.paint(id.with("row"), ui.window, ui.clip, paint);
    appearance
}

/// What the row shows after the editor ran.
pub(super) struct Content<'a> {
    pub label: &'a str,
    pub color: Color,
    /// Chevron rotation: a quarter turn when open.
    pub angle: f32,
    /// Text color resolved by the row's surface.
    pub text: Color,
}

pub(super) fn paint_content(
    ui: &mut Ui<'_>,
    id: Id,
    row: Row,
    content: Content<'_>,
    style: &crate::Style,
) {
    let rounding = style
        .color_picker
        .rounding
        .unwrap_or(CornerRadius::all(4.0));
    // Preserve the actual selected color when the row uses a hover gradient.
    let swatch = vec![rounded(row.swatch, content.color, rounding, style.border)];
    ui.context
        .paint(super::swatch_id(id), ui.window, ui.clip, swatch);
    let label = visible_label(content.label);
    let size = crate::components::font_size(style.font_size);
    let weight = style.typography.weights.control;
    let height = ui
        .context
        .measure_text(label, size, weight, f32::INFINITY)
        .y;
    let optical = ui.context.centered_line_offset(label, size, weight);
    let rect = row.rect;
    let caption_clip = Rect::from_min_max(
        rect.min,
        Vec2::new((row.swatch.min.x - 8.0).max(rect.min.x), rect.max.y),
    );
    ui.context.paint(
        id.with("caption"),
        ui.window,
        ui.clip.intersect(caption_clip),
        vec![Paint::Text {
            text: label.to_owned(),
            position: Vec2::new(
                rect.min.x + row.inset + 20.0,
                rect.center().y - height * 0.5 + optical,
            ),
            size,
            weight,
            wrap_width: f32::INFINITY,
            color: content.text,
        }],
    );
    let center = Vec2::new(rect.min.x + row.inset + 6.0, rect.center().y);
    let (sin, cos) = (content.angle.sin(), content.angle.cos());
    let rotate = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos) + center;
    let line = |start, end| {
        Paint::Shape(Shape::Line {
            start: rotate(start),
            end: rotate(end),
            width: 1.5,
            color: content.text,
        })
    };
    ui.context.paint(
        id.with("chevron"),
        ui.window,
        ui.clip,
        vec![
            line(Vec2::new(-2.0, -4.0), Vec2::new(2.0, 0.0)),
            line(Vec2::new(2.0, 0.0), Vec2::new(-2.0, 4.0)),
        ],
    );
    let ring = if ui.context.focus_visible(id) {
        style.focus_border
    } else {
        Border::NONE
    };
    ui.context.paint(
        id.with("focus"),
        ui.window,
        ui.clip,
        vec![rounded(rect, Color::TRANSPARENT, rounding, ring)],
    );
}
