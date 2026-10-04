use crate::{
    context::{native_chrome::NativeChrome, HitAction, HitRegion, Paint},
    vec2, Border, Color, Context, Id, Rect, Shape,
};

/// Windows-style native caption controls. Call after `Root::show` in each pass
/// and reserve `HEIGHT` above its content. Native drag/resize runs on mouse press.
/// On macOS the runner uses system decorations; this widget draws nothing.
pub struct TitleBar {
    title: String,
    maximized: bool,
    style: crate::TitleBarStyle,
}

#[derive(Default)]
pub struct TitleBarResponse {
    pub close: bool,
    pub minimize: bool,
    pub maximize: bool,
}

impl TitleBar {
    /// Client-area height to reserve. macOS uses the system title bar outside the client area.
    pub const HEIGHT: f32 = if cfg!(target_os = "macos") { 0.0 } else { 46.0 };
    pub const BUTTON_WIDTH: f32 = 46.0;
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            maximized: false,
            style: Default::default(),
        }
    }
    pub fn style(mut self, style: crate::TitleBarStyle) -> Self {
        self.style = style;
        self
    }
    pub fn maximized(mut self, value: bool) -> Self {
        self.maximized = value;
        self
    }
    pub fn show(self, context: &mut Context) -> TitleBarResponse {
        if cfg!(target_os = "macos") {
            context.native_chrome = None;
            return TitleBarResponse::default();
        }
        let style = context.style().clone();
        let mut component = style.title_bar;
        component.merge(self.style);
        let height = component.height.unwrap_or(Self::HEIGHT).max(0.0);
        let button_width = component
            .button_width
            .unwrap_or(Self::BUTTON_WIDTH)
            .max(0.0);
        let owner = Id::new("zaxis-root");
        let viewport = context.viewport();
        let rect = Rect::from_min_size(
            viewport.min,
            vec2(viewport.size().x, height.min(viewport.size().y)),
        );
        let controls = (rect.max.x - 3.0 * button_width).max(rect.min.x);
        context.native_chrome = Some(NativeChrome {
            owner,
            drag: Rect::from_min_max(rect.min, vec2(controls, rect.max.y)),
            resize_border: 5.0,
        });
        let mut title =
            super::appearance::Appearance::new(style.window_fill, Border::NONE, style.text_color);
        title.opacity = style.opacity;
        title.apply(component.surface);
        let title_font = super::font_size(component.font_size.unwrap_or(style.font_size));
        let title_weight = component
            .font_weight
            .unwrap_or(style.typography.weights.body);
        let mut paint = Vec::new();
        title.paint_shadow(rect, title.rounding, &mut paint);
        title.paint_body(rect, title.rounding, &style, title.blur, &mut paint);
        paint.push(Paint::Text {
            text: self.title,
            position: rect.min + vec2(16.0, (height - title_font * 1.25) * 0.5),
            size: title_font,
            weight: title_weight,
            wrap_width: (controls - rect.min.x - 32.0).max(0.0),
            color: title.text_color,
        });
        context.paint_blur(
            owner.with("title-blur"),
            owner,
            rect,
            crate::Blur::new(rect)
                .radius(title.blur)
                .corner_radius(title.rounding),
        );
        context.paint(owner.with("native-title"), owner, rect, paint);
        let mut output = TitleBarResponse::default();
        for index in 0..3 {
            let id = owner.with(("caption-control", index));
            let button = Rect::from_min_size(
                vec2(controls + index as f32 * button_width, rect.min.y),
                vec2(button_width, rect.size().y),
            );
            let hovered = context.hovered(id, owner, button, rect);
            let pressed = context.active(id);
            let mut paint = Vec::new();
            let state = crate::ControlState {
                enabled: true,
                hovered,
                pressed,
                focus: context.focus_visible(id),
                ..Default::default()
            };
            let fill = if index == 2 && (hovered || pressed) {
                style.error
            } else if pressed {
                style.button_pressed
            } else if hovered {
                style.button_hovered
            } else {
                Color::TRANSPARENT
            };
            let color = if index == 2 && (hovered || pressed) {
                style.on_error
            } else {
                style.text_color
            };
            let mut base = super::appearance::Appearance::new(fill, Border::NONE, color);
            base.opacity = style.opacity;
            base = super::appearance::resolve_control(
                &style,
                base,
                crate::HoverStyle::NONE,
                false,
                component.controls,
                state,
                fill,
            );
            let options = if context.palette_transition.is_some() {
                crate::TweenOptions::new(std::time::Duration::ZERO)
            } else {
                style.motion.hover.clone()
            };
            let appearance = context
                .transition_visible(
                    id.with(("appearance", context.style_revision)),
                    None,
                    base,
                    options,
                    true,
                )
                .value;
            appearance.paint_shadow(button, appearance.rounding, &mut paint);
            appearance.paint_body(
                button,
                appearance.rounding,
                &style,
                appearance.blur,
                &mut paint,
            );
            let color = appearance.text_color;
            let center = button.center();
            let mut line = |start, end| {
                paint.push(Paint::Shape(Shape::Line {
                    start,
                    end,
                    width: 1.0,
                    color,
                }))
            };
            match index {
                0 => line(center + vec2(-5.0, 0.0), center + vec2(5.0, 0.0)),
                1 => {
                    let icon = Rect::from_min_size(center - vec2(5.0, 5.0), vec2(10.0, 10.0));
                    if self.maximized {
                        line(icon.min + vec2(3.0, -3.0), icon.min + vec2(13.0, -3.0));
                        line(icon.min + vec2(13.0, -3.0), icon.min + vec2(13.0, 7.0));
                        line(icon.min + vec2(10.0, 7.0), icon.min + vec2(13.0, 7.0));
                    }
                    paint.push(Paint::Shape(Shape::Rect {
                        rect: icon,
                        fill: Color::TRANSPARENT,
                        rounding: 0.0.into(),
                        border: Border::new(1.0, color),
                    }));
                }
                _ => {
                    line(center + vec2(-5.0, -5.0), center + vec2(5.0, 5.0));
                    line(center + vec2(5.0, -5.0), center + vec2(-5.0, 5.0));
                }
            }
            if context.focus_visible(id) {
                paint.push(Paint::Shape(Shape::Rect {
                    rect: button.shrink(3.0),
                    fill: Color::TRANSPARENT,
                    rounding: 0.0.into(),
                    border: style.focus_border,
                }));
            }
            context.paint(id, owner, rect, paint);
            context.register_hit(HitRegion {
                id,
                window: owner,
                rect: button,
                clip: rect,
                action: HitAction::Activate,
            });
            if context.clicked(id) {
                match index {
                    0 => output.minimize = true,
                    1 => output.maximize = true,
                    _ => output.close = true,
                }
            }
        }
        output
    }
}
