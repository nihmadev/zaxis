use std::hash::Hash;

use crate::{
    context::{HitAction, HitRegion, Paint},
    Border, CornerRadius, Id, Padding, Vec2,
};

use super::appearance::Appearance;
use super::{font_size, visible_label, HoverStyle, Response, Ui, Widget};

/// A clickable button. A `##suffix` is part of its ID but hidden from its caption.
pub struct Button {
    text: String,
    id: Option<Id>,
    enabled: bool,
    min_size: Vec2,
    padding: Option<Padding>,
    rounding: Option<CornerRadius>,
    border: Option<Border>,
    selected: bool,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            id: None,
            enabled: true,
            min_size: Vec2::ZERO,
            padding: None,
            rounding: None,
            border: None,
            selected: false,
            blur: None,
            hover_style: None,
        }
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    pub fn enabled(mut self, value: bool) -> Self {
        self.enabled = value;
        self
    }
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
    pub fn corner_radius(self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding(radius.into())
    }
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size;
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn rounding(mut self, rounding: CornerRadius) -> Self {
        self.rounding = Some(rounding);
        self
    }
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }
    pub fn hover_style(mut self, style: HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
    }
    /// Override the theme's backdrop blur. Zero disables it for this button.
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }
}

impl Widget for Button {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let style = ui.style().clone();
        let size = font_size(style.font_size);
        let id = ui
            .scope
            .with(("button", self.id.unwrap_or_else(|| Id::new(&self.text))));
        let label = visible_label(&self.text);
        let text_size = ui.context.measure_text(label, size, f32::INFINITY);
        let padding = self.padding.unwrap_or(style.button_padding);
        let desired_size = (text_size + padding.size())
            .max(self.min_size)
            .max(Vec2::new(24.0, 24.0));
        let rect = ui.allocate_space(Vec2::new(
            desired_size.x.min(ui.available_width()),
            desired_size.y,
        ));
        let response = ui.response(id, rect, self.enabled);
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
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let fill = if response.pressed || self.selected {
            style.button_pressed
        } else {
            style.button_fill
        };
        let border = if response.focus_visible {
            style.focus_border
        } else {
            self.border.unwrap_or(style.border)
        };
        let hover = ui.animate_hover(
            response,
            preset,
            Appearance::new(
                fill,
                border,
                if self.enabled {
                    style.text_color
                } else {
                    style.muted_text
                },
            ),
            if self.selected {
                style.button_pressed
            } else {
                style.button_hovered
            },
        );
        let (blur, filter) = ui.control_blur(self.blur);
        let rounding = self.rounding.unwrap_or(CornerRadius::all(5.0));
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(rect)
                .radius(filter)
                .corner_radius(rounding),
        );
        let mut body = Vec::new();
        hover.paint_shadow(rect, rounding, &mut body);
        hover.paint_body(rect, rounding, &style, blur, &mut body);
        ui.context.paint(id.with("body"), ui.window, ui.clip, body);
        let text_rect = padding.inset(rect);
        ui.context.paint(
            id.with("caption"),
            ui.window,
            ui.clip.intersect(text_rect),
            vec![Paint::Text {
                text: label.to_owned(),
                position: rect.center() - text_size * 0.5,
                size,
                wrap_width: f32::INFINITY,
                color: hover.text_color,
            }],
        );
        response
    }
}

impl Ui<'_> {
    pub fn button_enabled(&mut self, enabled: bool, text: impl Into<String>) -> Response {
        self.add_enabled(enabled, Button::new(text))
    }
    pub fn selectable(&mut self, selected: bool, text: impl Into<String>) -> Response {
        self.add(Button::new(text).selected(selected))
    }
    pub fn selectable_value<T: PartialEq>(
        &mut self,
        selected: &mut T,
        value: T,
        text: impl Into<String>,
    ) -> Response {
        let mut response = self.selectable(*selected == value, text);
        if response.clicked() && *selected != value {
            *selected = value;
            response.changed = true;
        }
        response
    }
    /// Equal-width tabs with identities derived from their values.
    pub fn tab_bar<T: PartialEq + Hash, L: Into<String>>(
        &mut self,
        selected: &mut T,
        tabs: impl IntoIterator<Item = (T, L)>,
    ) -> Vec<Response> {
        let tabs: Vec<_> = tabs.into_iter().collect();
        let width = ((self.available_width()
            - self.style().spacing * tabs.len().saturating_sub(1) as f32)
            / tabs.len().max(1) as f32)
            .max(0.0);
        self.horizontal(|ui| {
            tabs.into_iter()
                .map(|(value, text)| {
                    let mut response = ui.add(
                        Button::new(text)
                            .id_source(&value)
                            .selected(*selected == value)
                            .min_size(Vec2::new(width, 38.0)),
                    );
                    if response.clicked() && *selected != value {
                        *selected = value;
                        response.changed = true;
                    }
                    response
                })
                .collect()
        })
    }
    pub fn button(&mut self, text: impl Into<String>) -> Response {
        self.add(Button::new(text))
    }
}
