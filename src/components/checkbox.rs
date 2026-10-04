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
    status: super::SemanticStatus,
    size: Option<f32>,
    rounding: Option<CornerRadius>,
    border: Option<Border>,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
    style: super::theme::CheckboxStyle,
    painter: Option<super::theme::painter::PaintHook<'a>>,
}

impl<'a> Checkbox<'a> {
    pub fn new(checked: &'a mut bool, text: impl Into<String>) -> Self {
        Self {
            checked,
            text: text.into(),
            id: None,
            enabled: true,
            status: Default::default(),
            size: None,
            rounding: None,
            border: None,
            blur: None,
            hover_style: None,
            style: Default::default(),
            painter: None,
        }
    }

    pub fn style(mut self, style: super::theme::CheckboxStyle) -> Self {
        self.style = style;
        self
    }
    /// Called once per part: CheckboxBody and CheckboxIndicator.
    pub fn painter(
        mut self,
        mode: super::theme::PaintMode,
        paint: impl Fn(&mut super::theme::Painter<'_>, super::theme::ControlPaint) + 'a,
    ) -> Self {
        self.painter = Some(super::theme::painter::PaintHook {
            mode,
            callback: Box::new(paint),
        });
        self
    }
    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Validation state: the border and a soft ring take the status color. Fields inherit
    /// the status of an enclosing [`super::Field`] unless this is set.
    pub fn status(mut self, status: super::SemanticStatus) -> Self {
        self.status = status;
        self
    }

    /// Override the square's side length in logical pixels.
    #[track_caller]
    pub fn size(mut self, size: f32) -> Self {
        self.size = super::sanitize::positive("Checkbox::size", size).or(self.size);
        self
    }

    #[deprecated(
        note = "use `.corner_radius(..)`; one name for the corner radius of every component"
    )]
    pub fn rounding(self, rounding: CornerRadius) -> Self {
        self.corner_radius(rounding)
    }
    pub fn corner_radius(mut self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding = Some(radius.into());
        self
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
        let mut component = style.checkbox;
        component.merge(self.style);
        let font_size = font_size(style.font_size);
        let id = ui
            .scope
            .with(("checkbox", self.id.unwrap_or_else(|| Id::new(&self.text))));
        let label = visible_label(&self.text);
        let available = ui.available_width();
        let side = self
            .size
            .or(component.size)
            .unwrap_or(font_size * 1.25)
            .max(10.0)
            .clamp(10.0, 128.0)
            .min(available);
        let gap = if label.is_empty() {
            0.0
        } else {
            component
                .gap
                .unwrap_or(style.spacing.max(0.0).clamp(0.0, 8.0))
        };
        let label_width = (available - side - gap).max(0.0);
        let text_size = if label.is_empty() {
            Vec2::ZERO
        } else {
            ui.context.measure_text(
                label,
                font_size,
                ui.style().typography.weights.control,
                label_width,
            )
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
        let status = ui.field_status(self.status);
        let mut state = super::theme::ControlState::from_response(response, *self.checked);
        state.status = status;
        let fill = if response.pressed {
            style.button_pressed
        } else if *self.checked {
            style.accent
        } else {
            style.button_fill
        };
        let mut base = Appearance::new(fill, self.border.unwrap_or(style.border), style.text_color);
        base.rounding = self.rounding.unwrap_or(CornerRadius::all(4.0));
        base.blur = self.blur.unwrap_or(0.0);
        base.opacity = style.opacity;
        base.shadow = style.elevation;
        base.status = ui.status_color(status);
        let mut hover = ui.animate_control(
            response,
            preset,
            self.hover_style.or(ui.hover_style).is_some(),
            component.body,
            state,
            base,
            if *self.checked {
                style.accent
            } else {
                style.button_hovered
            },
        );
        if let Some(v) = self.rounding {
            hover.rounding = v;
        }
        if let Some(v) = self.border.filter(|_| !state.focus) {
            hover.border = v;
        }
        if let Some(v) = self.blur {
            hover.blur = v;
        }
        let color = super::appearance::alpha(hover.text_color, hover.opacity);
        let (blur, filter) = ui.resolved_blur(
            hover.blur,
            self.blur.is_some() || component.body.has_blur_override(),
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
                crate::Blur::new(square)
                    .radius(filter)
                    .corner_radius(rounding),
            );
        }
        let mut indicator = Vec::new();
        let info = super::theme::ControlPaint {
            bounds: square,
            part: super::theme::PaintPart::CheckboxBody,
            style: hover.surface(),
            state,
            value: if *self.checked { 1.0 } else { 0.0 },
        };
        if let Some(h) = &self.painter {
            if h.mode != super::theme::PaintMode::After {
                h.run(&mut indicator, ui.clip_rect(), info);
            }
        }
        if self
            .painter
            .as_ref()
            .is_none_or(|h| h.mode != super::theme::PaintMode::Replace)
        {
            hover.paint_shadow(square, rounding, &mut indicator);
            hover.paint_body(square, rounding, &style, blur, &mut indicator);
        }
        if let Some(h) = &self.painter {
            if h.mode == super::theme::PaintMode::After {
                h.run(&mut indicator, ui.clip_rect(), info);
            }
        }
        let mut mark = Appearance::new(
            crate::Color::TRANSPARENT,
            Border::NONE,
            if self.enabled {
                style.on_accent
            } else {
                style.disabled_text
            },
        );
        mark.rounding = rounding;
        let mut mark_state = state;
        mark_state.focus = false;
        let mut mark = ui.animate_control(
            super::Response {
                id: id.with("mark"),
                ..response
            },
            HoverStyle::NONE,
            false,
            component.indicator,
            mark_state,
            mark,
            style.on_accent,
        );
        if state.focus {
            mark.apply(component.indicator.focus);
        }
        let info = super::theme::ControlPaint {
            part: super::theme::PaintPart::CheckboxIndicator,
            style: mark.surface(),
            ..info
        };
        if let Some(h) = &self.painter {
            if h.mode != super::theme::PaintMode::After {
                h.run(&mut indicator, ui.clip_rect(), info);
            }
        }

        if *self.checked
            && self
                .painter
                .as_ref()
                .is_none_or(|h| h.mode != super::theme::PaintMode::Replace)
        {
            mark.ring = None;
            mark.paint_shadow(square, mark.rounding, &mut indicator);
            mark.paint_body(square, mark.rounding, &style, mark.blur, &mut indicator);
            if mark.blur > 0.0 {
                ui.context.paint_blur(
                    id.with("mark-blur"),
                    ui.window,
                    ui.clip,
                    crate::Blur::new(square)
                        .radius(mark.blur)
                        .corner_radius(mark.rounding),
                );
            }
            let point = |x, y| square.min + Vec2::new(x, y) * side;
            let width = component.indicator_width.unwrap_or((side * 0.1).max(1.0));
            let color = super::appearance::alpha(mark.text_color, hover.opacity * mark.opacity);
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
        if let Some(h) = &self.painter {
            if h.mode == super::theme::PaintMode::After {
                h.run(&mut indicator, ui.clip_rect(), info);
            }
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
                    weight: ui.style().typography.weights.control,
                    wrap_width: label_width,
                    color,
                }],
            );
        }
        response
    }
}

impl Ui<'_> {
    #[deprecated(note = "use `ui.add(Checkbox::new(checked, text).enabled(enabled))`")]
    pub fn checkbox_enabled(
        &mut self,
        enabled: bool,
        checked: &mut bool,
        text: impl Into<String>,
    ) -> Response {
        self.add(Checkbox::new(checked, text).enabled(enabled))
    }
    /// Toggle a boolean by clicking its rounded square or label.
    pub fn checkbox(&mut self, checked: &mut bool, text: impl Into<String>) -> Response {
        self.add(Checkbox::new(checked, text))
    }
}
