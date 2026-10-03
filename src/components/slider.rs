use std::{hash::Hash, ops::RangeInclusive, panic::Location};

use winit::keyboard::KeyCode;

use crate::{
    context::{HitAction, HitRegion, Paint, SliderInput},
    Border, Color, CornerRadius, Id, Rect, Shape, Vec2,
};

use super::{font_size, visible_label, HoverStyle, Response, Ui, Widget};
mod paint;

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
    width: Option<f32>,
    color: Option<Color>,
    status: SliderStatus,
    source: &'static Location<'static>,
    text: Option<String>,
    suffix: String,
    precision: usize,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
    style: super::theme::SliderStyle,
    painter: Option<super::theme::painter::PaintHook<'a>>,
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
            width: None,
            color: None,
            status: SliderStatus::Normal,
            source: Location::caller(),
            text: None,
            suffix: String::new(),
            precision: 2,
            blur: None,
            hover_style: None,
            style: Default::default(),
            painter: None,
        }
    }

    pub fn style(mut self, style: super::theme::SliderStyle) -> Self {
        self.style = style;
        self
    }
    pub fn height(mut self, height: f32) -> Self {
        assert!(height.is_finite() && height > 0.0);
        self.style.height = Some(height);
        self
    }
    /// Called for SliderTrack (including its fill) and SliderThumb.
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
        self.width = Some(width);
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
        let mut component = style.slider;
        component.merge(self.style);
        let height = component.height.unwrap_or(28.0).max(0.0);
        let id = match self.id {
            Some(id) => ui.scope.with(("slider", id)),
            None => ui.auto_id(("slider", self.source)),
        };
        let rect = Rect::from_min_size(
            ui.layout.cursor,
            Vec2::new(
                self.width
                    .or(component.width)
                    .unwrap_or(200.0)
                    .min(ui.available_width()),
                height,
            ),
        );
        let mut response = ui.response(id, rect, self.enabled);
        let radius = component
            .thumb_radius
            .unwrap_or(8.0)
            .max(0.0)
            .min(rect.size().x * 0.5)
            .min(height * 0.5);
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
        ui.allocate_space(Vec2::new(width, height + label_height));
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
        let t = if span > 0.0 {
            (f64::from(value) - min) / span
        } else {
            0.0
        };
        let x = left + (right - left) * t as f32;
        let foreground = self.paint_parts(ui, response, &style, component, left, right, x, radius);
        if let Some((text, _)) = label {
            ui.context.paint(
                id.with("label"),
                ui.window,
                ui.clip,
                vec![Paint::Text {
                    text,
                    position: rect.min + Vec2::new(0.0, height + ui.layout.spacing),
                    size,
                    wrap_width: width,
                    color: foreground,
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
