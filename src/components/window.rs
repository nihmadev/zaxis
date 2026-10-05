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
    title_bar: bool,
    fit_content: bool,
    visual: Option<(f32, f32)>,
    on_top: bool,
    padding: Option<Padding>,
    rounding: Option<CornerRadius>,
    blur: Option<f32>,
    style: super::theme::WindowStyle,
    inherited_style: Option<super::Style>,
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
            title_bar: true,
            fit_content: false,
            visual: None,
            on_top: false,
            padding: None,
            rounding: None,
            blur: None,
            style: Default::default(),
            inherited_style: None,
        }
    }

    pub fn style(mut self, style: super::theme::WindowStyle) -> Self {
        self.style = style;
        self
    }
    pub(crate) fn effective_style(mut self, style: super::Style) -> Self {
        self.inherited_style = Some(style);
        self
    }
    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }
    #[track_caller]
    pub fn default_position(mut self, position: Vec2) -> Self {
        self.position = super::sanitize::finite_vec2("Window::default_position", position)
            .unwrap_or(self.position);
        self
    }
    /// Translate the entire panel for this pass without changing its retained
    /// drag position. Chrome, content, blur, clipping and hits move together.
    /// Unlike `default_position`, this is evaluated on every pass and may move
    /// outside the viewport for an exit animation.
    #[track_caller]
    pub fn offset(mut self, offset: Vec2) -> Self {
        self.offset = super::sanitize::finite_vec2("Window::offset", offset).unwrap_or(self.offset);
        self
    }
    #[track_caller]
    pub fn default_size(mut self, size: Vec2) -> Self {
        self.size = super::sanitize::size("Window::default_size", size).unwrap_or(self.size);
        self
    }
    #[track_caller]
    pub fn min_size(mut self, size: Vec2) -> Self {
        if let Some(size) = super::sanitize::size("Window::min_size", size) {
            self.min_size = size;
        }
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
    /// Size the window to its content (plus padding and title strip) on every pass, within
    /// `min_size`. The size follows the content with one pass of delay, so a row of text
    /// grows and shrinks with it; interactive resizing is then pointless, so the grip is hidden.
    pub fn fit_content(mut self, value: bool) -> Self {
        self.fit_content = value;
        self
    }
    /// Draw the whole window, chrome, blur and content, scaled about its center and
    /// faded to `opacity`, for entrance and exit animations. Hit regions and clips follow
    /// the same mapping. Layout is unaffected; `1.0, 1.0` is the ordinary window.
    #[track_caller]
    pub fn visual(mut self, scale: f32, opacity: f32) -> Self {
        if let Some(scale) = super::sanitize::positive("Window::visual", scale) {
            self.visual = Some((scale, opacity.clamp(0.0, 1.0)));
        }
        self
    }
    /// Keep the window above every window that is not `on_top`, also after those are
    /// clicked and raised: for heads-up panels that stay visible beside a menu.
    pub fn on_top(mut self, value: bool) -> Self {
        self.on_top = value;
        self
    }
    /// Show the title strip (default). `false` removes the strip, its title and its drag
    /// region: the content starts at the panel's top edge, the window may be smaller than
    /// 64 pixels, and the application moves it with [`Ui::drag_window`].
    pub fn title_bar(mut self, value: bool) -> Self {
        self.title_bar = value;
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = Some(padding);
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

    /// Blur the backdrop with sigma in logical pixels (0 disables, maximum 64).
    /// Window and title fills become translucent while blur is enabled.
    pub fn blur(mut self, radius: f32) -> Self {
        self.blur = Some(super::blur::normalize_radius(radius));
        self
    }

    pub fn show<R>(self, context: &mut Context, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        let inherited = self.inherited_style.is_some();
        let mut style = self
            .inherited_style
            .unwrap_or_else(|| context.style().clone());
        style.window.merge(self.style);
        let title_height = if self.title_bar {
            style
                .window
                .title_height
                .unwrap_or(style.title_height)
                .max(24.0)
        } else {
            0.0
        };
        let min_size = if self.title_bar {
            self.min_size
                .max(Vec2::new(64.0, 64.0))
                .max(Vec2::new(64.0, title_height + 32.0))
        } else {
            self.min_size.max(Vec2::splat(8.0))
        };
        let rect = context
            .window_state(
                self.id,
                Rect::from_min_size(self.position, self.size),
                min_size,
            )
            .translate(self.offset);
        context.windows.get_mut(&self.id).unwrap().displayed_rect = rect;
        context.set_window_on_top(self.id, self.on_top);
        let clip = rect.intersect(context.viewport());
        let title_rect = Rect::from_min_size(
            rect.min,
            Vec2::new(rect.size().x, title_height.min(rect.size().y)),
        );
        let mut body =
            super::appearance::Appearance::new(style.window_fill, style.border, style.text_color);
        body.rounding = style.rounding;
        body.blur = style.blur_radius;
        body.shadow = style.elevation;
        body.opacity = style.opacity;
        body.apply(style.window.body);
        if style.window.body.foreground.is_some() {
            style.text_color = body.text_color;
        }
        if let Some(v) = self.rounding {
            body.rounding = v;
        }
        if let Some(v) = self.blur {
            body.blur = v;
        }
        let rounding = body.rounding;
        let blur = body.blur;

        let visual = self
            .visual
            .filter(|(scale, opacity)| *scale != 1.0 || *opacity != 1.0);
        if visual.is_some() {
            context.begin_placement(self.id);
            context.visual_depth += 1;
            context.visual_clips.push((self.id, context.viewport()));
        }
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
        let mut title =
            super::appearance::Appearance::new(style.title_fill, Border::NONE, style.text_color);
        title.rounding = title_rounding;
        title.opacity = style.opacity;
        title.blur = blur;
        title.apply(style.window.title);
        let mut chrome = Vec::new();
        body.paint_shadow(rect, rounding, &mut chrome);
        body.paint_body(rect, rounding, &style, blur, &mut chrome);
        if self.title_bar {
            title.paint_shadow(title_rect, title.rounding, &mut chrome);
            title.paint_body(title_rect, title.rounding, &style, title.blur, &mut chrome);
        }
        // Window border remains above its title fill.
        chrome.push(Paint::Shape(
            Shape::rect(rect, Color::TRANSPARENT)
                .corner_radius(rounding)
                .border(body.border)
                .into(),
        ));
        context.paint(self.id.with("chrome"), self.id, context.viewport(), chrome);
        if self.title_bar {
            let title_font = font_size(style.window.title_font_size.unwrap_or(style.font_size));
            let title_weight = style
                .window
                .title_font_weight
                .unwrap_or(style.typography.weights.body);
            let title_clip = title_rect.shrink(style.border.width).intersect(clip);
            context.paint(
                self.id.with("title"),
                self.id,
                title_clip,
                vec![Paint::Text {
                    text: visible_label(&self.title).to_owned(),
                    position: title_rect.min
                        + Vec2::new(14.0, (title_height - title_font * 1.25) * 0.5),
                    size: title_font,
                    weight: title_weight,
                    wrap_width: f32::INFINITY,
                    color: super::appearance::alpha(title.text_color, title.opacity),
                }],
            );
        }
        context.register_hit(HitRegion {
            id: self.id,
            window: self.id,
            rect,
            clip,
            action: HitAction::Block,
        });
        if self.draggable && self.title_bar {
            context.register_hit(HitRegion {
                id: self.id.with("move"),
                window: self.id,
                rect: title_rect,
                clip,
                action: HitAction::Move,
            });
        }
        let scope = context.a11y_begin_layer(self.id, self.id, crate::AccessRole::Window, |node| {
            node.label(visible_label(&self.title)).clips();
        });
        let padding = self
            .padding
            .or(style.window.padding)
            .unwrap_or(style.window_padding);
        let mut bounds = padding.inset(Rect::from_min_max(
            Vec2::new(rect.min.x, title_rect.max.y),
            rect.max,
        ));
        let resizable = self.resizable && !self.fit_content;
        if resizable {
            bounds.max = bounds.max.min(rect.max - Vec2::splat(12.0)).max(bounds.min);
        }
        // The inner rectangular clip keeps child geometry away from rounded window corners.
        let content_clip = bounds.intersect(clip);
        let mut ui = Ui {
            flow: None,
            hover_style: None,
            local_style: (inherited || style.window.body.foreground.is_some())
                .then(|| std::sync::Arc::new(style.clone())),
            local_style_revision: 0,
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
        if self.fit_content {
            let wanted =
                (ui.layout.used + padding.size() + Vec2::new(0.0, title_height)).max(min_size);
            if let Some(state) = ui.context.windows.get_mut(&self.id) {
                if (state.rect.size() - wanted).abs().max_element() > 0.5 {
                    state.rect = Rect::from_min_size(state.rect.min, wanted);
                    ui.context.request_repaint();
                }
            }
        }
        if resizable {
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
        ui.context.a11y_end(scope, Some((rect, clip)));
        if let Some((scale, opacity)) = visual {
            let context = &mut *ui.context;
            context.visual_depth -= 1;
            context.visual_clips.pop();
            let placement = context.end_placement();
            let transform = crate::Transform::around(rect.center(), scale, Vec2::ZERO);
            let viewport = context.viewport();
            context.place_visual(placement, transform, opacity, viewport, opacity > 0.0);
        }
        result
    }
}
