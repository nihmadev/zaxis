use std::{hash::Hash, ops::RangeInclusive, panic::Location};

use winit::keyboard::KeyCode;

use crate::{
    context::{HitAction, HitRegion, Paint, SliderInput},
    Border, Color, CornerRadius, Id, Rect, Shape, Vec2,
};

use super::{font_size, visible_label, HoverStyle, Response, Ui, Widget};

/// Semantic accent for a slider. An explicit [`Slider::color`] takes precedence.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliderStatus {
    #[default]
    Normal,
    Success,
    Warning,
    Error,
}

/// A horizontal slider bound to a floating-point value.
/// Enabled values are clamped and snapped on every UI pass; disabled values are untouched.
/// Pointer capture continues outside the control. Tab focuses it; arrows adjust it,
/// PageUp/PageDown adjust by ten steps, and Home/End select the endpoints.
pub struct Slider<'a> {
    value: &'a mut f32,
    range: RangeInclusive<f32>,
    step: Option<f32>,
    id: Option<Id>,
    enabled: bool,
    width: f32,
    color: Option<Color>,
    status: SliderStatus,
    source: &'static Location<'static>,
    text: Option<String>,
    suffix: String,
    precision: usize,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
}

impl<'a> Slider<'a> {
    /// The range must have finite, ascending endpoints. Equal endpoints are allowed.
    #[track_caller]
    pub fn new(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        assert!(
            range.start().is_finite() && range.end().is_finite() && range.start() <= range.end(),
            "slider range must be finite and ascending"
        );
        Self {
            value,
            range,
            step: None,
            id: None,
            enabled: true,
            width: 200.0,
            color: None,
            status: SliderStatus::Normal,
            source: Location::caller(),
            text: None,
            suffix: String::new(),
            precision: 2,
            blur: None,
            hover_style: None,
        }
    }

    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }

    /// Display a caption and the current value below the track.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }
    pub fn precision(mut self, precision: usize) -> Self {
        assert!(precision <= 16, "slider precision must be at most 16");
        self.precision = precision;
        self
    }

    /// Snap to multiples of a positive, finite step relative to the range minimum.
    /// Both endpoints remain selectable even when the step does not divide the range.
    pub fn step(mut self, step: f32) -> Self {
        assert!(
            step.is_finite() && step > 0.0,
            "slider step must be finite and positive"
        );
        self.step = Some(step);
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }

    /// Preferred width in logical pixels, limited by the available layout width.
    pub fn width(mut self, width: f32) -> Self {
        assert!(
            width.is_finite() && width > 0.0,
            "slider width must be finite and positive"
        );
        self.width = width;
        self
    }

    /// Override the status accent on the filled track and thumb.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn status(mut self, status: SliderStatus) -> Self {
        self.status = status;
        self
    }
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }
    /// Apply the hover preset to the thumb, keeping the track's status accent.
    pub fn hover_style(mut self, style: HoverStyle) -> Self {
        self.hover_style = Some(style);
        self
    }

    fn normalized(&self, value: f64) -> f32 {
        let min = f64::from(*self.range.start());
        let max = f64::from(*self.range.end());
        let value = if value.is_nan() {
            min
        } else {
            value.clamp(min, max)
        };
        if value == min || value == max {
            return value as f32;
        }
        let value = if let Some(step) = self.step {
            min + ((value - min) / f64::from(step)).round() * f64::from(step)
        } else {
            value
        };
        value.clamp(min, max) as f32
    }
}

impl Widget for Slider<'_> {
    fn ui(mut self, ui: &mut Ui<'_>) -> Response {
        self.enabled &= ui.is_enabled();
        let style = ui.style().clone();
        let id = match self.id {
            Some(id) => ui.scope.with(("slider", id)),
            None => ui.auto_id(("slider", self.source)),
        };
        let rect = Rect::from_min_size(
            ui.layout.cursor,
            Vec2::new(self.width.min(ui.available_width()), 28.0),
        );
        let mut response = ui.response(id, rect, self.enabled);
        let radius = 8.0_f32.min(rect.size().x * 0.5);
        let left = rect.min.x + radius;
        let right = rect.max.x - radius;
        let min = f64::from(*self.range.start());
        let max = f64::from(*self.range.end());
        let span = max - min;
        let mut value = self.normalized(f64::from(*self.value));
        let input = ui.context.take_slider_input(id);
        if self.enabled {
            for event in input {
                let next = match event {
                    SliderInput::Pointer(pointer) if right > left => {
                        let t = f64::from(((pointer.x - left) / (right - left)).clamp(0.0, 1.0));
                        min + span * t
                    }
                    SliderInput::Key(KeyCode::Home) => min,
                    SliderInput::Key(KeyCode::End) => max,
                    SliderInput::Key(key) => {
                        let direction = match key {
                            KeyCode::ArrowRight | KeyCode::ArrowUp => 1.0,
                            KeyCode::ArrowLeft | KeyCode::ArrowDown => -1.0,
                            KeyCode::PageUp => 10.0,
                            KeyCode::PageDown => -10.0,
                            _ => 0.0,
                        };
                        let step = self.step.map(f64::from).unwrap_or(span / 100.0);
                        if self.step.is_some() {
                            let offset = (f64::from(value) - min) / step;
                            // Avoid skipping a tick because the bound f32 rounded slightly.
                            let nearest = offset.round();
                            let offset = if (offset - nearest).abs() < 1e-6 {
                                nearest
                            } else {
                                offset
                            };
                            let tick = if direction > 0.0 {
                                offset.floor()
                            } else {
                                offset.ceil()
                            };
                            min + (tick + direction) * step
                        } else {
                            f64::from(value) + direction * step
                        }
                    }
                    _ => f64::from(value),
                };
                value = self.normalized(next);
            }
            response.changed = *self.value != value;
            *self.value = value;
        }
        let size = font_size(style.font_size);
        let width = rect.size().x;
        let label = self.text.as_ref().map(|text| {
            let caption = visible_label(text);
            let separator = if caption.is_empty() { "" } else { ": " };
            let text = format!(
                "{caption}{separator}{:.*}{}",
                self.precision, *self.value, self.suffix
            );
            let text_size = ui.context.measure_text(&text, size, width);
            (text, text_size)
        });
        let label_height = label
            .as_ref()
            .map_or(0.0, |(_, text_size)| ui.layout.spacing + text_size.y);
        ui.allocate_space(Vec2::new(width, 28.0 + label_height));
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if self.enabled {
                HitAction::Slider
            } else {
                HitAction::Block
            },
        });
        let accent = if !self.enabled {
            style.muted_text
        } else {
            self.color.unwrap_or(match self.status {
                SliderStatus::Normal => style.text_color,
                SliderStatus::Success => Color::rgb(91, 184, 121),
                SliderStatus::Warning => Color::rgb(230, 177, 70),
                SliderStatus::Error => Color::rgb(220, 94, 94),
            })
        };
        let t = if span > 0.0 {
            (f64::from(value) - min) / span
        } else {
            0.0
        };
        let x = left + (right - left) * t as f32;
        let track = Rect::from_min_max(
            Vec2::new(left, rect.center().y - 3.0),
            Vec2::new(right, rect.center().y + 3.0),
        );
        let thumb = Rect::from_min_size(
            Vec2::new(x - radius, rect.center().y - radius),
            Vec2::splat(radius * 2.0),
        );
        let (blur, filter) = ui.control_blur(self.blur);
        ui.context.paint_blur(
            id.with("track-blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(track).radius(filter).corner_radius(3.0),
        );
        ui.context.paint_blur(
            id.with("thumb-blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(thumb).radius(filter).corner_radius(radius),
        );
        let preset = self
            .hover_style
            .or(ui.hover_style)
            .unwrap_or(style.hover_style);
        let hover = ui.animate_hover(
            response,
            preset,
            super::appearance::Appearance::new(
                if response.pressed {
                    style.button_pressed
                } else {
                    accent
                },
                style.border,
                if self.enabled {
                    style.text_color
                } else {
                    style.muted_text
                },
            ),
            accent,
        );
        let rounding = CornerRadius::all(radius);
        let mut body = vec![
            Paint::Shape(Shape::Rect {
                rect: track,
                fill: style.backdrop_fill(style.button_fill, blur),
                rounding: CornerRadius::all(3.0),
                border: Border::NONE,
            }),
            Paint::Shape(Shape::Rect {
                rect: Rect::from_min_max(track.min, Vec2::new(x, track.max.y)),
                fill: style.backdrop_fill(accent, blur),
                rounding: CornerRadius::all(3.0),
                border: Border::NONE,
            }),
        ];
        hover.paint_shadow(thumb, rounding, &mut body);
        hover.paint_body(thumb, rounding, &style, blur, &mut body);
        body.push(Paint::Shape(Shape::Rect {
            rect,
            fill: Color::TRANSPARENT,
            rounding: CornerRadius::all(5.0),
            border: if response.focus_visible {
                style.focus_border
            } else {
                Border::NONE
            },
        }));
        ui.context.paint(id.with("body"), ui.window, ui.clip, body);
        if let Some((text, _)) = label {
            ui.context.paint(
                id.with("label"),
                ui.window,
                ui.clip,
                vec![Paint::Text {
                    text,
                    position: rect.min + Vec2::new(0.0, 28.0 + ui.layout.spacing),
                    size,
                    wrap_width: width,
                    color: hover.text_color,
                }],
            );
        }
        response
    }
}

impl Ui<'_> {
    #[track_caller]
    pub fn slider_enabled(
        &mut self,
        enabled: bool,
        value: &mut f32,
        range: RangeInclusive<f32>,
    ) -> Response {
        self.add_enabled(enabled, Slider::new(value, range))
    }
    #[track_caller]
    pub fn slider_labeled(
        &mut self,
        value: &mut f32,
        range: RangeInclusive<f32>,
        text: impl Into<String>,
    ) -> Response {
        self.add(Slider::new(value, range).text(text))
    }
    #[track_caller]
    pub fn slider(&mut self, value: &mut f32, range: RangeInclusive<f32>) -> Response {
        self.add(Slider::new(value, range))
    }
}
