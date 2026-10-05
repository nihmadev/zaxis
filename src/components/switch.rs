use std::hash::Hash;

use crate::{
    context::{HitAction, HitRegion, Paint},
    Border, CornerRadius, Id, Rect, Shape, Vec2,
};

use super::appearance::{alpha, Appearance};
use super::{font_size, visible_label, HoverStyle, Response, Ui, Widget};

/// A boolean on/off control: a pill track with a sliding thumb and optional label.
/// Like [`Checkbox`](super::Checkbox) it edits an application-owned `&mut bool`;
/// the label is clickable and supports the hidden `##suffix` ID convention.
pub struct Switch<'a> {
    on: &'a mut bool,
    text: String,
    id: Option<Id>,
    enabled: bool,
    status: super::SemanticStatus,
    size: Option<f32>,
    rounding: Option<CornerRadius>,
    border: Option<Border>,
    hover_style: Option<HoverStyle>,
    style: super::theme::SwitchStyle,
}

impl<'a> Switch<'a> {
    pub fn new(on: &'a mut bool, text: impl Into<String>) -> Self {
        Self {
            on,
            text: text.into(),
            id: None,
            enabled: true,
            status: Default::default(),
            size: None,
            rounding: None,
            border: None,
            hover_style: None,
            style: Default::default(),
        }
    }

    pub fn style(mut self, style: super::theme::SwitchStyle) -> Self {
        self.style = style;
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
    /// Override the track height in logical pixels; the width keeps its ratio.
    #[track_caller]
    pub fn size(mut self, size: f32) -> Self {
        self.size = super::sanitize::positive("Switch::size", size).or(self.size);
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
    pub fn hover_style(mut self, style: HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let enabled = self.enabled && ui.is_enabled();
        let style = ui.style().clone();
        let mut component = style.switch;
        component.merge(self.style);
        let font_size = font_size(style.font_size);
        let id = ui
            .scope
            .with(("switch", self.id.unwrap_or_else(|| Id::new(&self.text))));
        let label = visible_label(&self.text);
        let available = ui.available_width();
        let height = self
            .size
            .or(component.height)
            .unwrap_or(font_size * 1.25)
            .clamp(10.0, 128.0)
            .min(available);
        let width = match (self.size, component.width) {
            (None, Some(width)) => width,
            _ => height * 1.8,
        }
        .clamp(height, 256.0)
        .min(available);
        let gap = if label.is_empty() {
            0.0
        } else {
            component
                .gap
                .unwrap_or(style.spacing.max(0.0).clamp(0.0, 8.0))
        };
        let label_width = (available - width - gap).max(0.0);
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
            (width + gap + text_size.x).min(available),
            height.max(text_size.y),
        ));
        let mut response = ui.response(id, rect, enabled);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if enabled {
                HitAction::Activate
            } else {
                HitAction::Block
            },
        });
        if response.clicked {
            *self.on = !*self.on;
            response.changed = true;
        }
        let on = *self.on;
        ui.a11y(id, rect, crate::AccessRole::Switch, |node| {
            node.label(label).toggled(on).disabled(!enabled).clicks(id);
        });
        let track = Rect::from_min_size(
            rect.min + Vec2::new(0.0, (rect.size().y - height) * 0.5),
            Vec2::new(width, height),
        );
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let status = ui.field_status(self.status);
        let mut state = super::theme::ControlState::from_response(response, on);
        state.status = status;
        let fill = if response.pressed {
            style.button_pressed
        } else if on {
            style.accent
        } else {
            style.button_fill
        };
        let mut base = Appearance::new(fill, self.border.unwrap_or(style.border), style.text_color);
        base.rounding = self.rounding.unwrap_or(CornerRadius::all(height * 0.5));
        base.opacity = style.opacity;
        base.shadow = style.elevation;
        base.status = ui.status_color(status);
        let mut body = ui.animate_control(
            response,
            preset,
            self.hover_style.or(ui.hover_style).is_some(),
            component.track,
            state,
            base,
            if on {
                style.accent
            } else {
                style.button_hovered
            },
        );
        if let Some(v) = self.rounding {
            body.rounding = v;
        }
        if let Some(v) = self.border.filter(|_| !state.focus) {
            body.border = v;
        }
        let text_color = alpha(body.text_color, body.opacity);
        let rounding = body.rounding;
        let mut paint = Vec::new();
        body.paint_shadow(track, rounding, &mut paint);
        body.paint_body(track, rounding, &style, 0.0, &mut paint);

        // The thumb slides along one animated track; reduced motion snaps it.
        let motion = style.motion.hover.clone();
        let position = ui.transition_property(
            id.with("thumb-position"),
            rect,
            if on { 1.0_f32 } else { 0.0 },
            motion,
        );
        let inset = component
            .thumb_inset
            .unwrap_or(height * 0.15)
            .clamp(0.0, height * 0.4);
        let radius = (height * 0.5 - inset).max(1.0);
        let travel = (width - height).max(0.0);
        let center = Vec2::new(
            track.min.x + height * 0.5 + travel * position.clamp(0.0, 1.0),
            track.min.y + height * 0.5,
        );
        let thumb_rect =
            Rect::from_min_max(center - Vec2::splat(radius), center + Vec2::splat(radius));
        let mut thumb_state = state;
        thumb_state.focus = false;
        let thumb_base = Appearance::new(
            if on {
                style.on_accent
            } else {
                style.text_color
            },
            Border::NONE,
            style.on_accent,
        );
        let mut thumb = ui.animate_control(
            Response {
                id: id.with("thumb"),
                ..response
            },
            HoverStyle::NONE,
            false,
            component.thumb,
            thumb_state,
            thumb_base,
            if on {
                style.on_accent
            } else {
                style.text_color
            },
        );
        thumb.rounding = CornerRadius::all(radius);
        thumb.paint_shadow(thumb_rect, thumb.rounding, &mut paint);
        paint.push(Paint::Shape(Shape::Circle {
            center,
            radius,
            fill: alpha(thumb.fill.start, thumb.opacity * body.opacity),
            border: Border {
                color: alpha(thumb.border.color, thumb.opacity * body.opacity),
                ..thumb.border
            },
        }));
        ui.context
            .paint(id.with("track"), ui.window, ui.clip, paint);
        if !label.is_empty() {
            let label_rect = Rect::from_min_max(Vec2::new(track.max.x + gap, rect.min.y), rect.max);
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
                    color: text_color,
                }],
            );
        }
        response
    }
}

impl Ui<'_> {
    #[deprecated(note = "use `ui.add(Switch::new(on, text).enabled(enabled))`")]
    pub fn switch_enabled(
        &mut self,
        enabled: bool,
        on: &mut bool,
        text: impl Into<String>,
    ) -> Response {
        self.add(Switch::new(on, text).enabled(enabled))
    }
    /// Toggle a boolean by clicking its sliding track or label.
    pub fn switch(&mut self, on: &mut bool, text: impl Into<String>) -> Response {
        self.add(Switch::new(on, text))
    }
}
