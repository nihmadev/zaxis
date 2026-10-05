use std::time::Duration;

use super::{Response, Ui, Widget};
use crate::{
    context::HitRegion, AccessAction, AccessActionKind, Border, Color, CornerRadius, Id, Padding,
};

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

/// How a tooltip finds the node of the widget it belongs to.
enum Target {
    /// By the widget's id among the nodes described before the tooltip.
    Find,
    /// The first node of the wrapped widget with the requests that waited for it, or
    /// `None` when that widget described nothing.
    Wrapped(Option<(usize, Vec<AccessAction>)>),
}

fn asks(request: &AccessAction) -> bool {
    matches!(
        request,
        AccessAction::ShowTooltip | AccessAction::HideTooltip
    )
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
        self.attach(ui, response, Target::Find);
    }

    /// The text is the description of the widget for a screen reader, shown or not, unless
    /// the widget has one already or the text only repeats its name. A widget whose tooltip
    /// can be shown also accepts the requests to show and to hide it.
    fn describe(text: &str, ui: &mut Ui<'_>, id: Id, target: Target, showable: bool) {
        let (node, requests) = match target {
            Target::Wrapped(None) => return,
            Target::Wrapped(Some((node, requests))) => (ui.context.a11y_node_mut(node), requests),
            Target::Find => {
                let waiting = ui.context.a11y_requests(asks).into_iter();
                let requests = waiting.filter(|(node, _)| *node == id);
                let requests = requests.map(|(_, request)| request).collect();
                (ui.context.a11y_node_of(id), requests)
            }
        };
        let Some(node) = node else {
            return;
        };
        let named = node.label.as_deref() == Some(text)
            || node.role.is_text() && node.value.as_deref() == Some(text);
        if node.description.is_none() && !named {
            node.description(text);
        }
        if !showable {
            return;
        }
        node.action(AccessActionKind::ShowTooltip);
        for request in requests {
            let shown = &mut ui.context.tooltips.shown;
            match request {
                AccessAction::ShowTooltip => *shown = Some(id),
                AccessAction::HideTooltip if *shown == Some(id) => *shown = None,
                _ => continue,
            }
            ui.context.request_repaint();
        }
    }

    fn attach(self, ui: &mut Ui<'_>, response: Response, target: Target) {
        // A tooltip that is switched off for this widget says nothing; one the theme
        // switches off for everyone is still the widget's description.
        if self.enabled == Some(false) || self.text.trim().is_empty() {
            return;
        }
        let mut style = self.style.unwrap_or_else(|| ui.style().tooltip.clone());
        style.enabled = self.enabled.unwrap_or(style.enabled);
        if ui.context.a11y_on() {
            Self::describe(&self.text, ui, response.id, target, style.enabled);
        }
        if !style.enabled {
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
        let before = ui.context.a11y_wrap(asks);
        let response = self.widget.ui(ui);
        let wrapped = ui.context.a11y_wrapped(before);
        self.tooltip.attach(ui, response, Target::Wrapped(wrapped));
        response
    }
}

impl Ui<'_> {
    pub fn tooltip(&mut self, response: Response, text: impl Into<String>) {
        Tooltip::new(text).show(self, response);
    }
}
