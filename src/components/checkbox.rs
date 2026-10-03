use std::hash::Hash;

use crate::{
    context::{HitAction, HitRegion, Paint},
    Border, CornerRadius, Id, Rect, Shape, Vec2,
};

use super::appearance::Appearance;
use super::{font_size, visible_label, HoverStyle, Response, Ui, Widget};

/// A boolean control with a checkmark inside a rounded square when checked.
/// The label is clickable and supports the same hidden `##suffix` IDs as buttons.
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    text: String,
    id: Option<Id>,
    enabled: bool,
    size: Option<f32>,
    rounding: Option<CornerRadius>,
    border: Option<Border>,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<String>) -> Self {
        Self {
            checked,
            text: text.into(),
            id: None,
            enabled: true,
            size: None,
            rounding: None,
            border: None,
            blur: None,
            hover_style: None,
        }
    }

    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Override the square's side length in logical pixels.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn rounding(mut self, rounding: CornerRadius) -> Self {
        self.rounding = Some(rounding);
        self
    }
    pub fn corner_radius(self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding(radius.into())
    }

    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }
    pub fn hover_style(mut self, style: HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let style = ui.style().clone();
        let font_size = font_size(style.font_size);
        let id = ui
            .scope
            .with(("checkbox", self.id.unwrap_or_else(|| Id::new(&self.text))));
        let label = visible_label(&self.text);
        let available = ui.available_width();
        let side = self
            .size
            .unwrap_or(font_size * 1.25)
            .max(10.0)
            .min(128.0)
            .min(available);
        let gap = if label.is_empty() {
            0.0
        } else {
            style.spacing.max(0.0).min(8.0)
        };
        let label_width = (available - side - gap).max(0.0);
        let text_size = if label.is_empty() {
            Vec2::ZERO
        } else {
            ui.context.measure_text(label, font_size, label_width)
        };
        let rect = ui.allocate_space(Vec2::new(
            (side + gap + text_size.x).min(available),
            side.max(text_size.y),
        ));
        let mut response = ui.response(id, rect, self.enabled);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if self.enabled {
                HitAction::Activate
            } else {
                HitAction::Block
            },
        });
        if response.clicked {
            *self.checked = !*self.checked;
            response.changed = true;
        }
        let square = Rect::from_min_size(
            rect.min + Vec2::new(0.0, (rect.size().y - side) * 0.5),
            Vec2::splat(side),
        );
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let fill = if response.pressed {
            style.button_pressed
        } else {
            style.button_fill
        };
        let border = if response.focus_visible {
            style.focus_border
        } else {
            self.border.unwrap_or(style.border)
        };
        let color = if self.enabled {
            style.text_color
        } else {
            style.muted_text
        };
        let hover = ui.animate_hover(
            response,
            preset,
            Appearance::new(fill, border, color),
            style.button_hovered,
        );
        let color = hover.text_color;
        let (blur, filter) = ui.control_blur(self.blur);
        let rounding = self.rounding.unwrap_or(CornerRadius::all(4.0));
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(square)
                .radius(filter)
                .corner_radius(rounding),
        );
        let mut indicator = Vec::new();
        hover.paint_shadow(square, rounding, &mut indicator);
        hover.paint_body(square, rounding, &style, blur, &mut indicator);
        if *self.checked {
            let point = |x, y| square.min + Vec2::new(x, y) * side;
            let width = (side * 0.1).max(1.0);
            indicator.extend([
                Paint::Shape(Shape::Line {
                    start: point(0.23, 0.51),
                    end: point(0.43, 0.71),
                    width,
                    color,
                }),
                Paint::Shape(Shape::Line {
                    start: point(0.43, 0.71),
                    end: point(0.78, 0.29),
                    width,
                    color,
                }),
            ]);
        }
        ui.context
            .paint(id.with("indicator"), ui.window, ui.clip, indicator);
        if !label.is_empty() {
            let label_rect =
                Rect::from_min_max(Vec2::new(square.max.x + gap, rect.min.y), rect.max);
            ui.context.paint(
                id.with("caption"),
                ui.window,
                ui.clip.intersect(label_rect),
                vec![Paint::Text {
                    text: label.to_owned(),
                    position: label_rect.min + Vec2::new(0.0, (rect.size().y - text_size.y) * 0.5),
                    size: font_size,
                    wrap_width: label_width,
                    color,
                }],
            );
        }
        response
    }
}

impl Ui<'_> {
    pub fn checkbox_enabled(
        &mut self,
        enabled: bool,
        checked: &mut bool,
        text: impl Into<String>,
    ) -> Response {
        self.add_enabled(enabled, Checkbox::new(checked, text))
    }
    /// Toggle a boolean by clicking its rounded square or label.
    pub fn checkbox(&mut self, checked: &mut bool, text: impl Into<String>) -> Response {
        self.add(Checkbox::new(checked, text))
    }
}
