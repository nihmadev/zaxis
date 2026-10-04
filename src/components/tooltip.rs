use std::time::Duration;

use super::{Response, Ui, Widget};
use crate::{context::HitRegion, Border, Color, CornerRadius, Padding};

/// Defaults inherited by every tooltip. Measurements use logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct TooltipStyle {
    pub enabled: bool,
    pub delay: Duration,
    pub max_width: f32,
    pub padding: Padding,
    pub rounding: CornerRadius,
    pub fill: Option<Color>,
    pub text_color: Option<Color>,
    pub border: Option<Border>,
}

impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            delay: Duration::from_millis(350),
            max_width: 320.0,
            padding: Padding::symmetric(10.0, 6.0),
            rounding: CornerRadius::all(6.0),
            fill: None,
            text_color: None,
            border: None,
        }
    }
}

/// A passive text overlay. It never takes focus or intercepts pointer input.
/// Attach to a response with `show`, or wrap any widget with `wrap`.
pub struct Tooltip {
    pub(crate) text: String,
    enabled: Option<bool>,
    style: Option<TooltipStyle>,
}

pub struct TooltipWidget<W> {
    tooltip: Tooltip,
    widget: W,
}

impl Tooltip {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            enabled: None,
            style: None,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn style(mut self, style: TooltipStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn wrap<W: Widget>(self, widget: W) -> TooltipWidget<W> {
        TooltipWidget {
            tooltip: self,
            widget,
        }
    }

    /// Call in the same UI scope as the target response.
    /// Disabled controls can still explain why they are unavailable.
    pub fn show(self, ui: &mut Ui<'_>, response: Response) {
        let mut style = self.style.unwrap_or_else(|| ui.style().tooltip.clone());
        style.enabled = self.enabled.unwrap_or(style.enabled);
        if !style.enabled || self.text.trim().is_empty() {
            return;
        }
        let anchor = response.id.with("tooltip-anchor");
        // Temporary routing geometry follows deferred layout, scrolling and visual
        // transforms. It is removed before input hits are published to the host.
        let hit = HitRegion {
            id: anchor,
            window: ui.window,
            rect: response.rect,
            clip: ui.clip,
            action: crate::context::HitAction::Block,
        };
        if !ui.attach_tooltip_anchor(response.id, hit) {
            ui.context.register_hit(hit);
        }
        let font_size = super::font_size(ui.style().font_size);
        let fill = style.fill.unwrap_or(ui.style().window_fill);
        let color = style.text_color.unwrap_or(ui.style().text_color);
        let border = style.border.unwrap_or(ui.style().border);
        ui.context
            .tooltips
            .pending
            .push(crate::context::tooltip::TooltipRequest {
                target: response.id,
                anchor,
                text: self.text,
                style,
                font_size,
                font_weight: ui.style().typography.weights.body,
                fill,
                color,
                border,
            });
    }
}

impl<W: Widget> Widget for TooltipWidget<W> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let response = self.widget.ui(ui);
        self.tooltip.show(ui, response);
        response
    }
}

impl Ui<'_> {
    pub fn tooltip(&mut self, response: Response, text: impl Into<String>) {
        Tooltip::new(text).show(self, response);
    }
}
