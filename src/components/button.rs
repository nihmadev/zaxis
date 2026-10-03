use std::hash::Hash;

use crate::{
    context::{HitAction, HitRegion, Paint},
    Border, CornerRadius, Id, Padding, Vec2,
};

use super::appearance::Appearance;
use super::{font_size, visible_label, HoverStyle, Response, Ui, Widget};

/// A clickable button. A `##suffix` is part of its ID but hidden from its caption.
pub struct Button<F = fn(&mut crate::Painter<'_>, crate::ControlPaint)> {
    text: String,
    id: Option<Id>,
    enabled: bool,
    min_size: Vec2,
    padding: Option<Padding>,
    rounding: Option<CornerRadius>,
    border: Option<Border>,
    selected: bool,
    status: crate::SemanticStatus,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
    style: super::theme::ButtonStyle,
    painter: Option<super::theme::painter::PaintCallbackHook<F>>,
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
            status: Default::default(),
            blur: None,
            hover_style: None,
            style: Default::default(),
            painter: None,
        }
    }
}
impl<F: Fn(&mut crate::Painter<'_>, crate::ControlPaint)> Button<F> {
    pub fn style(mut self, style: super::theme::ButtonStyle) -> Self {
        self.style = style;
        self
    }
    pub fn painter<G: Fn(&mut crate::Painter<'_>, crate::ControlPaint)>(
        self,
        mode: crate::PaintMode,
        paint: G,
    ) -> Button<G> {
        Button {
            text: self.text,
            id: self.id,
            enabled: self.enabled,
            min_size: self.min_size,
            padding: self.padding,
            rounding: self.rounding,
            border: self.border,
            selected: self.selected,
            status: self.status,
            blur: self.blur,
            hover_style: self.hover_style,
            style: self.style,
            painter: Some(super::theme::painter::PaintCallbackHook {
                mode,
                callback: paint,
            }),
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
    pub fn status(mut self, status: crate::SemanticStatus) -> Self {
        self.status = status;
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
    /// Explicitly blur this button's backdrop. Background blur is not inherited.
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }
}

impl<F: Fn(&mut crate::Painter<'_>, crate::ControlPaint)> Widget for Button<F> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let style = ui.style().clone();
        let mut component = style.button;
        component.merge(self.style);
        let size = font_size(component.font_size.unwrap_or(style.font_size));
        let id = ui
            .scope
            .with(("button", self.id.unwrap_or_else(|| Id::new(&self.text))));
        let label = visible_label(&self.text);
        let text_size = ui.context.measure_text(label, size, f32::INFINITY);
        let padding = self
            .padding
            .or(component.padding)
            .unwrap_or(style.button_padding);
        let desired_size = (text_size + padding.size())
            .max(self.min_size)
            .max(component.min_size.unwrap_or(Vec2::ZERO))
            .max(Vec2::new(24.0, style.control_height));
        let rect = ui.allocate_space(Vec2::new(
            desired_size.x.min(ui.available_width()),
            desired_size.y.min(ui.available_height()),
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
        let mut state = super::theme::ControlState::from_response(response, self.selected);
        state.status = self.status;
        let fill = if !self.enabled {
            style.button_fill
        } else if response.pressed {
            style.button_pressed
        } else if self.selected {
            style.selected_fill
        } else {
            style.button_fill
        };
        let mut base = Appearance::new(
            fill,
            self.border.unwrap_or(style.border),
            if self.selected {
                style.selected_text
            } else {
                style.text_color
            },
        );
        if self.enabled {
            match self.status {
                crate::SemanticStatus::Normal => {}
                crate::SemanticStatus::Success => {
                    base.fill = crate::Gradient::new(style.success, style.success);
                    base.text_color = style.on_success;
                }
                crate::SemanticStatus::Warning => {
                    base.fill = crate::Gradient::new(style.warning, style.warning);
                    base.text_color = style.on_warning;
                }
                crate::SemanticStatus::Error => {
                    base.fill = crate::Gradient::new(style.error, style.error);
                    base.text_color = style.on_error;
                }
            }
        }
        base.rounding = self.rounding.unwrap_or(CornerRadius::all(5.0));
        base.blur = self.blur.unwrap_or(0.0);
        base.opacity = style.opacity;
        base.shadow = style.elevation;
        let mut hover = ui.animate_control(
            response,
            preset,
            self.hover_style.or(ui.hover_style).is_some(),
            component.surface,
            state,
            base,
            if self.status != crate::SemanticStatus::Normal {
                base.fill.start
            } else if self.selected {
                style.selected_fill
            } else {
                style.button_hovered
            },
        );
        // Geometry builders are invariant across interaction states.
        if let Some(v) = self.rounding {
            hover.rounding = v;
        }
        if let Some(v) = self.blur {
            hover.blur = v;
        }
        if let Some(v) = self.border.filter(|_| !state.focus) {
            hover.border = v;
        }
        let (blur, filter) = ui.resolved_blur(
            hover.blur,
            self.blur.is_some() || component.surface.has_blur_override(),
        );
        let rounding = hover.rounding;
        if self
            .painter
            .as_ref()
            .is_none_or(|h| h.mode != crate::PaintMode::Replace)
        {
            ui.context.paint_blur(
                id.with("blur"),
                ui.window,
                ui.clip,
                crate::Blur::new(rect)
                    .radius(filter)
                    .corner_radius(rounding),
            );
        }
        let mut body = Vec::new();
        let info = super::theme::ControlPaint {
            bounds: rect,
            part: super::theme::PaintPart::Button,
            style: hover.surface(),
            state,
            value: if self.selected { 1.0 } else { 0.0 },
        };
        if let Some(hook) = &self.painter {
            if hook.mode != super::theme::PaintMode::After {
                hook.run(&mut body, ui.clip_rect(), info);
            }
        }
        if self
            .painter
            .as_ref()
            .is_none_or(|h| h.mode != super::theme::PaintMode::Replace)
        {
            hover.paint_shadow(rect, rounding, &mut body);
            hover.paint_body(rect, rounding, &style, blur, &mut body);
        }
        if let Some(hook) = &self.painter {
            if hook.mode == super::theme::PaintMode::After {
                hook.run(&mut body, ui.clip_rect(), info);
            }
        }
        ui.context.paint(id.with("body"), ui.window, ui.clip, body);
        // Fixed-height Grid/Table cells may be shorter than the theme's button.
        // Fit the allocation first, then reduce padding before clipping its label.
        let spare = (rect.size() - text_size).max(Vec2::ZERO);
        let psize = padding.size();
        let x = if psize.x > 0.0 {
            (spare.x / psize.x).min(1.0)
        } else {
            1.0
        };
        let y = if psize.y > 0.0 {
            (spare.y / psize.y).min(1.0)
        } else {
            1.0
        };
        let text_rect = Padding {
            left: padding.left * x,
            right: padding.right * x,
            top: padding.top * y,
            bottom: padding.bottom * y,
        }
        .inset(rect)
        .intersect(rect);
        let optical = ui.context.centered_line_offset(label, size);
        ui.context.paint(
            id.with("caption"),
            ui.window,
            ui.clip.intersect(text_rect),
            vec![Paint::Text {
                text: label.to_owned(),
                position: rect.center() - text_size * 0.5 + Vec2::new(0.0, optical),
                size,
                wrap_width: f32::INFINITY,
                color: super::appearance::alpha(hover.text_color, hover.opacity),
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
