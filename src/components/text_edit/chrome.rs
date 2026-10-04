//! The field surface shared by both modes: resolved metrics, hit region, hover/focus/
//! status appearance, blur and body.
use super::*;
use crate::components::{
    appearance::{alpha, Appearance},
    theme::{ControlState, TextEditStyle},
};

/// Style and metrics resolved for one pass.
pub(super) struct Look {
    pub style: crate::Style,
    pub component: TextEditStyle,
    pub size: f32,
    pub font: crate::text::TextFont,
    pub padding: Padding,
}

pub(super) struct Chrome {
    pub color: Color,
    pub caret: Color,
}

impl TextEdit<'_> {
    pub(super) fn look(&self, ui: &Ui<'_>) -> Look {
        let mut style = ui.style().clone();
        let mut component = style.text_edit;
        component.merge(self.style);
        if let Some(v) = component.cursor_width {
            style.text_edit_cursor_width = v.max(0.0);
        }
        if let Some(v) = component.blink_interval {
            style.text_edit_blink_interval = v;
        }
        let size = crate::components::font_size(
            self.size
                .or(component.font_size)
                .unwrap_or(style.text_edit_font_size),
        );
        let weight = component
            .font_weight
            .unwrap_or(style.typography.weights.body);
        let font = crate::text::TextFont::new(
            weight,
            self.family
                .or(component.font_family)
                .unwrap_or_default(),
            self.tabular.unwrap_or(false),
        );
        let padding = self.padding.or_else(|| match self.area {
            Some(_) => Some(component.area_padding.unwrap_or(Padding::symmetric(12.0, 8.0))),
            None => component.padding,
        });
        let padding = padding.unwrap_or(style.text_edit_padding);
        Look {
            style,
            component,
            size,
            font,
            padding,
        }
    }

    /// Register the hit region and paint blur and body; returns the resolved appearance.
    pub(super) fn paint_chrome(
        &self,
        ui: &mut Ui<'_>,
        id: Id,
        rect: Rect,
        response: Response,
        look: &Look,
    ) -> Chrome {
        let (style, component) = (&look.style, &look.component);
        let mut state_style = ControlState::from_response(response, false);
        state_style.focus = self.enabled && response.has_focus;
        let status = ui.field_status(self.status);
        state_style.status = status;
        let mut base = Appearance::new(
            self.fill.unwrap_or(style.text_edit_fill),
            Border::NONE,
            self.text_color.unwrap_or(style.text_color),
        );
        base.rounding = self
            .rounding
            .or(component.rounding)
            .unwrap_or(style.text_edit_rounding);
        base.blur = 0.0;
        base.opacity = style.opacity;
        base.status = ui.status_color(status);
        let explicit = self.hover_style.or(ui.hover_style);
        let hovered = self
            .hovered_fill
            .or(self.fill)
            .unwrap_or(style.text_edit_hovered);
        let preset = explicit.unwrap_or(crate::HoverStyle::fill(hovered));
        let mut appearance = ui.animate_control(
            response,
            preset,
            explicit.is_some(),
            component.surface,
            state_style,
            base,
            hovered,
        );
        if let Some(v) = self.rounding {
            appearance.rounding = v;
        }
        let color = alpha(
            self.text_color.unwrap_or(appearance.text_color),
            appearance.opacity,
        );
        let caret = component.caret.unwrap_or(color);
        ui.context.register_hit(HitRegion {
            id,
            window: ui.window,
            rect,
            clip: ui.clip,
            action: if self.enabled {
                HitAction::TextEdit
            } else {
                HitAction::Block
            },
        });
        let (_, filter) = ui.resolved_blur(appearance.blur, component.surface.has_blur_override());
        ui.context.paint_blur(
            id.with("blur"),
            ui.window,
            ui.clip,
            crate::Blur::new(rect)
                .radius(filter)
                .corner_radius(appearance.rounding),
        );
        let mut body = Vec::new();
        appearance.paint_shadow(rect, appearance.rounding, &mut body);
        appearance.paint_body(rect, appearance.rounding, style, appearance.blur, &mut body);
        ui.context.paint(id.with("body"), ui.window, ui.clip, body);
        Chrome { color, caret }
    }

    pub(super) fn selection_fill(&self, look: &Look) -> Color {
        self.selection_color
            .or(look.component.selection)
            .unwrap_or(look.style.text_edit_selection)
    }

    pub(super) fn placeholder_fill(&self, look: &Look) -> Color {
        self.placeholder_color
            .or(look.component.placeholder)
            .unwrap_or(look.style.text_edit_placeholder)
    }
}

impl TextEditState {
    pub(super) fn new(text: &str, blink_interval: Duration) -> Self {
        Self {
            buffer: EditBuffer {
                cursor: text.len(),
                anchor: text.len(),
                ..Default::default()
            },
            scroll: 0.0,
            fingerprint: Fingerprint::of(text),
            history: EditHistory::default(),
            word_drag: None,
            drag_paragraphs: false,
            focused: false,
            blink_interval,
            preedit: None,
            area: None,
        }
    }

    /// The application changed the string: keep what is still valid, drop the rest.
    pub(super) fn text_replaced(&mut self, text: &str) {
        self.buffer.clamp(text);
        self.preedit = None;
        self.history = EditHistory::default();
        self.word_drag = None;
    }
}
