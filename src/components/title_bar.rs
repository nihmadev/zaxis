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
        }
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
        let owner = Id::new("zaxis-root");
        let viewport = context.viewport();
        let rect = Rect::from_min_size(
            viewport.min,
            vec2(viewport.size().x, Self::HEIGHT.min(viewport.size().y)),
        );
        let controls = (rect.max.x - 3.0 * Self::BUTTON_WIDTH).max(rect.min.x);
        context.native_chrome = Some(NativeChrome {
            owner,
            drag: Rect::from_min_max(rect.min, vec2(controls, rect.max.y)),
            resize_border: 5.0,
        });
        context.paint(
            owner.with("native-title"),
            owner,
            rect,
            vec![
                Paint::Shape(Shape::Rect {
                    rect,
                    fill: context.style().window_fill,
                    rounding: 0.0.into(),
                    border: Border::NONE,
                }),
                Paint::Text {
                    text: self.title,
                    position: rect.min + vec2(16.0, 13.0),
                    size: 16.0,
                    wrap_width: (controls - rect.min.x - 32.0).max(0.0),
                    color: context.style().text_color,
                },
            ],
        );
        let mut output = TitleBarResponse::default();
        for index in 0..3 {
            let id = owner.with(("caption-control", index));
            let button = Rect::from_min_size(
                vec2(controls + index as f32 * Self::BUTTON_WIDTH, rect.min.y),
                vec2(Self::BUTTON_WIDTH, rect.size().y),
            );
            let hovered = context.hovered(id, owner, button, rect);
            let pressed = context.active(id);
            let mut paint = Vec::new();
            if hovered || pressed {
                let fill = if index == 2 {
                    if pressed {
                        Color::rgb(190, 30, 40)
                    } else {
                        Color::rgb(232, 17, 35)
                    }
                } else if pressed {
                    Color::gray(65)
                } else {
                    Color::gray(76)
                };
                paint.push(Paint::Shape(Shape::Rect {
                    rect: button,
                    fill,
                    rounding: 0.0.into(),
                    border: Border::NONE,
                }));
            }
            let color = if hovered && index == 2 {
                Color::WHITE
            } else {
                context.style().text_color
            };
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
                    border: context.style().focus_border,
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
