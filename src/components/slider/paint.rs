use super::*;
use crate::components::{appearance::Appearance, theme::*};
impl Slider<'_> {
    pub(super) fn paint_parts(
        &self,
        ui: &mut Ui<'_>,
        response: Response,
        style: &crate::Style,
        component: SliderStyle,
        left: f32,
        right: f32,
        x: f32,
        radius: f32,
    ) -> Color {
        let rect = response.rect;
        let mut state = ControlState::from_response(response, false);
        state.status = match self.status {
            SliderStatus::Normal => SemanticStatus::Normal,
            SliderStatus::Success => SemanticStatus::Success,
            SliderStatus::Warning => SemanticStatus::Warning,
            SliderStatus::Error => SemanticStatus::Error,
        };
        let accent = if !self.enabled {
            style.disabled_text
        } else {
            self.color.unwrap_or(match self.status {
                SliderStatus::Normal => style.accent,
                SliderStatus::Success => style.success,
                SliderStatus::Warning => style.warning,
                SliderStatus::Error => style.error,
            })
        };
        let thickness = component
            .track_height
            .unwrap_or(6.0)
            .max(0.0)
            .min(rect.size().y);
        let track = Rect::from_min_max(
            Vec2::new(left, rect.center().y - thickness * 0.5),
            Vec2::new(right, rect.center().y + thickness * 0.5),
        );
        let thumb = Rect::from_min_size(
            Vec2::new(x - radius, rect.center().y - radius),
            Vec2::splat(radius * 2.0),
        );
        let filled = Rect::from_min_max(track.min, Vec2::new(x, track.max.y));
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let mut normal = Appearance::new(style.button_fill, Border::NONE, style.text_color);
        normal.rounding = CornerRadius::all(thickness * 0.5);
        normal.blur = self.blur.unwrap_or(0.0);
        normal.opacity = style.opacity;
        let track_style = ui.animate_control(
            Response {
                id: response.id.with("track"),
                ..response
            },
            HoverStyle::NONE,
            true,
            component.track,
            state,
            normal,
            style.button_hovered,
        );
        normal.fill = crate::Gradient::new(accent, accent);
        let mut fill_style = ui.animate_control(
            Response {
                id: response.id.with("filled"),
                ..response
            },
            HoverStyle::NONE,
            true,
            component.fill,
            state,
            normal,
            accent,
        );
        normal.rounding = CornerRadius::all(radius);
        normal.border = style.border;
        normal.shadow = style.elevation;
        let mut thumb_style = ui.animate_control(
            response,
            preset,
            self.hover_style.or(ui.hover_style).is_some(),
            component.thumb,
            state,
            normal,
            accent,
        );
        // Explicit accent is invariant under status and interaction colors.
        if self.color.is_some() {
            fill_style.fill = crate::Gradient::new(accent, accent);
            thumb_style.fill = fill_style.fill;
        }
        let mut body = Vec::new();
        for (part, bounds, appearance) in [
            (PaintPart::SliderTrack, track, track_style),
            (PaintPart::SliderThumb, thumb, thumb_style),
        ] {
            let info = ControlPaint {
                bounds,
                part,
                style: appearance.surface(),
                state,
                value: *self.value,
            };
            if let Some(h) = &self.painter {
                if h.mode != PaintMode::After {
                    h.run(&mut body, ui.clip_rect(), info);
                }
            }
            if self
                .painter
                .as_ref()
                .is_none_or(|h| h.mode != PaintMode::Replace)
            {
                let (_, filter) = ui.resolved_blur(
                    appearance.blur,
                    self.blur.is_some()
                        || component.track.has_blur_override()
                        || component.thumb.has_blur_override(),
                );
                ui.context.paint_blur(
                    response.id.with(("part-blur", part as u8)),
                    ui.window,
                    ui.clip,
                    crate::Blur::new(bounds)
                        .radius(filter)
                        .corner_radius(appearance.rounding),
                );
                appearance.paint_shadow(bounds, appearance.rounding, &mut body);
                appearance.paint_body(
                    bounds,
                    appearance.rounding,
                    style,
                    appearance.blur,
                    &mut body,
                );
                if part == PaintPart::SliderTrack {
                    fill_style.paint_body(
                        filled,
                        fill_style.rounding,
                        style,
                        fill_style.blur,
                        &mut body,
                    );
                }
            }
            if let Some(h) = &self.painter {
                if h.mode == PaintMode::After {
                    h.run(&mut body, ui.clip_rect(), info);
                }
            }
        }
        if state.focus {
            body.push(Paint::Shape(
                Shape::rect(rect, Color::TRANSPARENT)
                    .corner_radius(style.rounding)
                    .border(style.focus_border)
                    .into(),
            ));
        }
        ui.context
            .paint(response.id.with("body"), ui.window, ui.clip, body);
        crate::components::appearance::alpha(thumb_style.text_color, thumb_style.opacity)
    }
}
