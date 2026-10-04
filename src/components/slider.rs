use std::{hash::Hash, ops::RangeInclusive, panic::Location};

use winit::keyboard::KeyCode;

use crate::{
    context::{HitAction, HitRegion, Paint, SliderInput},
    Border, Color, CornerRadius, Id, Rect, Vec2,
};

use super::{font_size, visible_label, HoverStyle, Numeric, Response, SemanticStatus, Ui, Widget};
mod paint;

/// Space between a caption above the track and the track.
const CAPTION_GAP: f32 = 4.0;

/// Semantic accent for a slider; the same type every control uses for validation.
/// An explicit [`Slider::color`] takes precedence.
pub type SliderStatus = SemanticStatus;

/// A horizontal slider bound to a number: `f32` by default, any [`Numeric`] type works.
/// Integers snap to whole numbers and show no decimals without further configuration.
/// Enabled values are clamped and snapped on every UI pass; disabled values are untouched.
/// Pointer capture continues outside the control. Tab focuses it; arrows adjust it,
/// PageUp/PageDown adjust by ten steps, and Home/End select the endpoints.
pub struct Slider<'a, T: Numeric = f32> {
    value: &'a mut T,
    range: RangeInclusive<T>,
    step: Option<T>,
    id: Option<Id>,
    enabled: bool,
    width: Option<f32>,
    color: Option<Color>,
    status: SemanticStatus,
    source: &'static Location<'static>,
    text: Option<String>,
    caption_above: bool,
    suffix: String,
    precision: Option<usize>,
    blur: Option<f32>,
    hover_style: Option<HoverStyle>,
    style: super::theme::SliderStyle,
    painter: Option<super::theme::painter::PaintHook<'a>>,
}

impl<'a, T: Numeric> Slider<'a, T> {
    /// Equal endpoints are allowed. Descending endpoints are swapped; a range with a
    /// non-finite end becomes `0..=1`.
    #[track_caller]
    pub fn new(value: &'a mut T, range: RangeInclusive<T>) -> Self {
        let (start, end) = (*range.start(), *range.end());
        let range = if !start.finite() || !end.finite() {
            crate::context::invalid_value(
                "Slider::new",
                format!("expected a finite range, got {start}..={end}; using 0..=1"),
            );
            T::from_f64(0.0)..=T::from_f64(1.0)
        } else if start > end {
            crate::context::invalid_value(
                "Slider::new",
                format!("expected an ascending range, got {start}..={end}; ends swapped"),
            );
            end..=start
        } else {
            range
        };
        Self {
            value,
            range,
            step: None,
            id: None,
            enabled: true,
            width: None,
            color: None,
            status: SemanticStatus::Normal,
            source: Location::caller(),
            text: None,
            caption_above: false,
            suffix: String::new(),
            precision: None,
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
    #[track_caller]
    pub fn height(mut self, height: f32) -> Self {
        self.style.height =
            super::sanitize::positive("Slider::height", height).or(self.style.height);
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
    /// Put the caption on a line above the track: the text left-aligned and the value with
    /// its suffix right-aligned, instead of one `caption: value` line below it.
    pub fn caption_above(mut self, above: bool) -> Self {
        self.caption_above = above;
        self
    }
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }
    /// Decimals shown for floats (default 2, at most 16). Integers never show decimals.
    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = Some(precision.min(16));
        self
    }

    /// Snap to multiples of a positive, finite step relative to the range minimum;
    /// integers default to one. Both endpoints remain selectable even when the step
    /// does not divide the range. A step that is not positive is ignored.
    #[track_caller]
    pub fn step(mut self, step: T) -> Self {
        if step.positive() {
            self.step = Some(step);
        } else {
            crate::context::invalid_value(
                "Slider::step",
                format!("expected a finite step > 0, got {step}; ignored"),
            );
        }
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[deprecated(
        note = "use `.enabled(!disabled)`; `enabled` is the one way to set a control's availability"
    )]
    pub fn disabled(self, disabled: bool) -> Self {
        self.enabled(!disabled)
    }

    /// Preferred width in logical pixels, limited by the available layout width.
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = super::sanitize::positive("Slider::width", width).or(self.width);
        self
    }

    /// Override the status accent on the filled track and thumb.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn status(mut self, status: SemanticStatus) -> Self {
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

    /// The explicit step, or one for integer types.
    fn step_f64(&self) -> Option<f64> {
        self.step.map(T::to_f64).or(T::INTEGER.then_some(1.0))
    }

    fn normalized(&self, value: f64) -> T {
        let (start, end) = (*self.range.start(), *self.range.end());
        let (min, max) = (start.to_f64(), end.to_f64());
        let value = if value.is_nan() {
            min
        } else {
            value.clamp(min, max)
        };
        if value == min {
            return start;
        }
        if value == max {
            return end;
        }
        let value = if let Some(step) = self.step_f64() {
            min + ((value - min) / step).round() * step
        } else {
            value
        };
        T::from_f64(value).normalized(start, end)
    }
}

impl<T: Numeric> Widget for Slider<'_, T> {
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
        let size = font_size(style.font_size);
        let weight = ui.style().typography.weights.control;
        let width = self
            .width
            .or(component.width)
            .unwrap_or(200.0)
            .min(ui.available_width());
        let precision = self.precision.unwrap_or(if T::INTEGER { 0 } else { 2 });
        // The caption fixes the line height; the value is formatted after input.
        let header = self
            .text
            .as_ref()
            .filter(|_| self.caption_above)
            .map(|text| {
                let caption = visible_label(text).to_owned();
                let line = ui.context.measure_text(&caption, size, weight, width).y;
                (caption, line)
            });
        let header_height = header.as_ref().map_or(0.0, |(_, line)| line + CAPTION_GAP);
        let rect = Rect::from_min_size(
            ui.layout.cursor + Vec2::new(0.0, header_height),
            Vec2::new(width, height),
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
        let min = self.range.start().to_f64();
        let max = self.range.end().to_f64();
        let span = max - min;
        let mut value = self.normalized(self.value.to_f64());
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
                        let step = self.step_f64().unwrap_or(span / 100.0);
                        if self.step_f64().is_some() {
                            let offset = (value.to_f64() - min) / step;
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
                            value.to_f64() + direction * step
                        }
                    }
                    _ => value.to_f64(),
                };
                value = self.normalized(next);
            }
            response.changed = !self.value.same(value);
            *self.value = value;
        }
        let label = self
            .text
            .as_ref()
            .filter(|_| !self.caption_above)
            .map(|text| {
                let caption = visible_label(text);
                let separator = if caption.is_empty() { "" } else { ": " };
                let text = format!(
                    "{caption}{separator}{}{}",
                    self.value.formatted(Some(precision)),
                    self.suffix
                );
                let text_size = ui.context.measure_text(&text, size, weight, width);
                (text, text_size)
            });
        let label_height = label
            .as_ref()
            .map_or(0.0, |(_, text_size)| ui.layout.spacing + text_size.y);
        ui.allocate_space(Vec2::new(width, header_height + height + label_height));
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
            (value.to_f64() - min) / span
        } else {
            0.0
        };
        let x = left + (right - left) * t as f32;
        let foreground = self.paint_parts(ui, response, &style, component, left, right, x, radius);
        if let Some((caption, _)) = header {
            let value = format!("{}{}", self.value.formatted(Some(precision)), self.suffix);
            let value_size = ui.context.measure_text(&value, size, weight, width);
            let top = rect.min - Vec2::new(0.0, header_height);
            ui.context.paint(
                id.with("header"),
                ui.window,
                ui.clip,
                vec![
                    Paint::Text {
                        text: caption,
                        position: top,
                        size,
                        weight,
                        wrap_width: width,
                        color: foreground,
                    },
                    Paint::Text {
                        text: value,
                        position: top + Vec2::new((width - value_size.x).max(0.0), 0.0),
                        size,
                        weight,
                        wrap_width: width,
                        color: foreground,
                    },
                ],
            );
        }
        if let Some((text, _)) = label {
            ui.context.paint(
                id.with("label"),
                ui.window,
                ui.clip,
                vec![Paint::Text {
                    text,
                    position: rect.min + Vec2::new(0.0, height + ui.layout.spacing),
                    size,
                    weight,
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
    #[deprecated(note = "use `ui.add(Slider::new(value, range).enabled(enabled))`")]
    pub fn slider_enabled<T: Numeric>(
        &mut self,
        enabled: bool,
        value: &mut T,
        range: RangeInclusive<T>,
    ) -> Response {
        self.add(Slider::new(value, range).enabled(enabled))
    }
    #[track_caller]
    pub fn slider_labeled<T: Numeric>(
        &mut self,
        value: &mut T,
        range: RangeInclusive<T>,
        text: impl Into<String>,
    ) -> Response {
        self.add(Slider::new(value, range).text(text))
    }
    #[track_caller]
    pub fn slider<T: Numeric>(&mut self, value: &mut T, range: RangeInclusive<T>) -> Response {
        self.add(Slider::new(value, range))
    }
}
