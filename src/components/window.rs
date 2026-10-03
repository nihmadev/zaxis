use crate::{
    context::{HitAction, HitRegion, Paint},
    layout::LayoutCursor,
    Border, Color, Context, CornerRadius, Id, Layout, Padding, Rect, Shape, Vec2,
};

use super::{font_size, visible_label, Ui};

/// A draggable window with a bottom-right resize handle and retained geometry.
/// Position and size builders set initial values; later interaction is kept by ID.
pub struct Window {
    title: String,
    id: Id,
    position: Vec2,
    offset: Vec2,
    size: Vec2,
    min_size: Vec2,
    draggable: bool,
    resizable: bool,
    padding: Option<Padding>,
    rounding: Option<CornerRadius>,
    blur: Option<f32>,
}

impl Window {
    pub fn new(title: impl Into<String>) -> Self {
        let title = title.into();
        Self {
            id: Id::new(("window", &title)),
            title,
            position: Vec2::new(40.0, 40.0),
            offset: Vec2::ZERO,
            size: Vec2::new(380.0, 240.0),
            min_size: Vec2::new(160.0, 100.0),
            draggable: true,
            resizable: true,
            padding: None,
            rounding: None,
            blur: None,
        }
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }
    pub fn default_position(mut self, position: Vec2) -> Self {
        self.position = position;
        self
    }
    /// Translate the entire panel for this pass without changing its retained
    /// drag position. Chrome, content, blur, clipping and hits move together.
    /// Unlike `default_position`, this is evaluated on every pass and may move
    /// outside the viewport for an exit animation.
    pub fn offset(mut self, offset: Vec2) -> Self {
        assert!(offset.is_finite(), "window offset must be finite");
        self.offset = offset;
        self
    }
    pub fn default_size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = size.max(Vec2::new(64.0, 64.0));
        self
    }
    pub fn draggable(mut self, value: bool) -> Self {
        self.draggable = value;
        self
    }
    pub fn resizable(mut self, value: bool) -> Self {
        self.resizable = value;
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
        self
    }
    pub fn rounding(mut self, rounding: CornerRadius) -> Self {
        self.rounding = Some(rounding);
        self
    }
    pub fn corner_radius(self, radius: impl Into<CornerRadius>) -> Self {
        self.rounding(radius.into())
    }

    /// Blur the backdrop with sigma in logical pixels (0 disables, maximum 64).
    /// Window and title fills become translucent while blur is enabled.
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }

    pub fn show<R>(self, context: &mut Context, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let style = context.style().clone();
        let title_height = style.title_height.max(24.0);
        let min_size = self.min_size.max(Vec2::new(64.0, title_height + 32.0));
        let rect = context
            .window_state(
                self.id,
                Rect::from_min_size(self.position, self.size),
                min_size,
            )
            .translate(self.offset);
        context.windows.get_mut(&self.id).unwrap().displayed_rect = rect;
        let clip = rect.intersect(context.viewport());
        let title_rect = Rect::from_min_size(
            rect.min,
            Vec2::new(rect.size().x, title_height.min(rect.size().y)),
        );
        let rounding = self.rounding.unwrap_or(style.rounding);
        let blur = super::blur::normalize_radius(self.blur.unwrap_or(style.blur_radius));
        context.paint_blur(
            self.id.with("blur"),
            self.id,
            clip,
            crate::Blur::new(rect).radius(blur).corner_radius(rounding),
        );
        let title_rounding = CornerRadius {
            bottom_left: 0.0,
            bottom_right: 0.0,
            ..rounding
        };
        context.paint(
            self.id.with("chrome"),
            self.id,
            clip,
            vec![
                Paint::Shape(Shape::Rect {
                    rect,
                    fill: style.backdrop_fill(style.window_fill, blur),
                    rounding,
                    border: Border::NONE,
                }),
                Paint::Shape(Shape::Rect {
                    rect: title_rect,
                    fill: style.backdrop_fill(style.title_fill, blur),
                    rounding: title_rounding,
                    border: Border::NONE,
                }),
                Paint::Shape(Shape::Rect {
                    rect,
                    fill: Color::TRANSPARENT,
                    rounding,
                    border: style.border,
                }),
            ],
        );
        let title_clip = title_rect.shrink(style.border.width).intersect(clip);
        context.paint(
            self.id.with("title"),
            self.id,
            title_clip,
            vec![Paint::Text {
                text: visible_label(&self.title).to_owned(),
                position: title_rect.min
                    + Vec2::new(14.0, (title_height - style.font_size * 1.25) * 0.5),
                size: font_size(style.font_size),
                wrap_width: f32::INFINITY,
                color: style.text_color,
            }],
        );
        context.register_hit(HitRegion {
            id: self.id,
            window: self.id,
            rect,
            clip,
            action: HitAction::Block,
        });
        if self.draggable {
            context.register_hit(HitRegion {
                id: self.id.with("move"),
                window: self.id,
                rect: title_rect,
                clip,
                action: HitAction::Move,
            });
        }
        let padding = self.padding.unwrap_or(style.window_padding);
        let mut bounds = padding.inset(Rect::from_min_max(
            Vec2::new(rect.min.x, title_rect.max.y),
            rect.max,
        ));
        if self.resizable {
            bounds.max = bounds.max.min(rect.max - Vec2::splat(12.0)).max(bounds.min);
        }
        // The inner rectangular clip keeps child geometry away from rounded window corners.
        let content_clip = bounds.intersect(clip);
        let mut ui = Ui {
            flow: None,
            hover_style: None,
            context,
            window: self.id,
            scope: self.id.with("content"),
            sequence: 0,
            clip: content_clip,
            layout: LayoutCursor::new(bounds, Layout::Vertical, style.spacing.max(0.0)),
            enabled: true,
            backdrop_blur: blur,
        };
        ui.begin_layout(crate::Align::Start);
        let result = build(&mut ui);
        ui.finish_layout();
        if self.resizable {
            let grip = Rect::from_min_size(rect.max - Vec2::splat(18.0), Vec2::splat(18.0));
            let color = style.muted_text;
            ui.context.paint(
                self.id.with("grip"),
                self.id,
                clip,
                vec![
                    Paint::Shape(Shape::Line {
                        start: rect.max - Vec2::new(13.0, 5.0),
                        end: rect.max - Vec2::new(5.0, 13.0),
                        width: 1.0,
                        color,
                    }),
                    Paint::Shape(Shape::Line {
                        start: rect.max - Vec2::new(9.0, 5.0),
                        end: rect.max - Vec2::new(5.0, 9.0),
                        width: 1.0,
                        color,
                    }),
                ],
            );
            ui.context.register_hit(HitRegion {
                id: self.id.with("resize"),
                window: self.id,
                rect: grip,
                clip,
                action: HitAction::Resize,
            });
        }
        result
    }
}
