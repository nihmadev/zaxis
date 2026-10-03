use super::Ui;
use crate::{
    context::{HitAction, HitRegion},
    layout::LayoutCursor,
    Border, Context, Id, Layout, Padding,
};

/// The application's client area, without an internal title bar or resize grip.
/// Follows the viewport on resize/DPI changes and stays below floating windows.
#[derive(Default)]
pub struct Root {
    padding: Option<Padding>,
    blur: Option<f32>,
}

impl Root {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Blur the application backdrop, using the same policy as `Window`.
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }

    /// Build one root per UI pass. Returns the closure's result.
    pub fn show<R>(self, context: &mut Context, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let id = Id::new("zaxis-root");
        let rect = context.viewport();
        let mut style = context.style().clone();
        context.root_state(id);
        let mut body =
            super::appearance::Appearance::new(style.window_fill, Border::NONE, style.text_color);
        body.opacity = style.opacity;
        body.blur = style.blur_radius;
        body.apply(style.window.body);
        if style.window.body.foreground.is_some() {
            style.text_color = body.text_color;
        }
        let blur = self.blur.unwrap_or(body.blur);
        context.paint_blur(
            id.with("blur"),
            id,
            rect,
            crate::Blur::new(rect).radius(blur),
        );
        body.rounding = 0.0.into();
        let mut paint = Vec::new();
        body.paint_body(rect, body.rounding, &style, blur, &mut paint);
        context.paint(id.with("background"), id, rect, paint);
        context.register_hit(HitRegion {
            id,
            window: id,
            rect,
            clip: rect,
            action: HitAction::Block,
        });
        let bounds = self
            .padding
            .or(style.window.padding)
            .unwrap_or(style.window_padding)
            .inset(rect);
        let mut ui = Ui {
            context,
            window: id,
            scope: id.with("content"),
            sequence: 0,
            clip: bounds.intersect(rect),
            layout: LayoutCursor::new(bounds, Layout::Vertical, style.spacing.max(0.0)),
            enabled: true,
            backdrop_blur: blur,
            hover_style: None,
            local_style: style
                .window
                .body
                .foreground
                .is_some()
                .then(|| std::sync::Arc::new(style.clone())),
            local_style_revision: 0,
            flow: None,
        };
        ui.begin_layout(crate::Align::Start);
        let result = build(&mut ui);
        ui.finish_layout();
        result
    }
}
