use super::Ui;
use crate::{
    context::{HitAction, HitRegion, Paint},
    layout::LayoutCursor,
    Border, Context, Id, Layout, Padding, Shape,
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
        let style = context.style().clone();
        context.root_state(id);
        let blur = self.blur.unwrap_or(style.blur_radius);
        context.paint_blur(
            id.with("blur"),
            id,
            rect,
            crate::Blur::new(rect).radius(blur),
        );
        context.paint(
            id.with("background"),
            id,
            rect,
            vec![Paint::Shape(Shape::Rect {
                rect,
                fill: style.backdrop_fill(style.window_fill, blur),
                rounding: 0.0.into(),
                border: Border::NONE,
            })],
        );
        context.register_hit(HitRegion {
            id,
            window: id,
            rect,
            clip: rect,
            action: HitAction::Block,
        });
        let bounds = self.padding.unwrap_or(style.window_padding).inset(rect);
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
            flow: None,
        };
        ui.begin_layout(crate::Align::Start);
        let result = build(&mut ui);
        ui.finish_layout();
        result
    }
}
