use std::time::Duration;

use crate::{Color, Context};

/// A transient notification stacked in the bottom-right corner of the viewport.
///
/// Toasts are passive overlays: they paint above windows, never take focus or intercept
/// the pointer, appear with a short fade and slide, and fade out when their time is up.
/// Queue one from anywhere with [`Context::toast`] (for example when a button was
/// clicked); the context keeps and paints it for its lifetime. Unset colors come from the
/// current style: window fill, hairline border, text and muted text.
///
/// ```
/// # let mut context = zaxis::Context::new();
/// context.toast(zaxis::Toast::new("Configurations").content("Saved aim.cfg"));
/// assert_eq!(context.toast_count(), 1);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Toast {
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) duration: Duration,
    pub(crate) width: f32,
    pub(crate) corner_radius: f32,
    pub(crate) fill: Option<Color>,
    pub(crate) border: Option<Color>,
    pub(crate) title_color: Option<Color>,
    pub(crate) content_color: Option<Color>,
    pub(crate) title_size: f32,
    pub(crate) content_size: f32,
}

impl Toast {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            content: String::new(),
            duration: Duration::from_secs(4),
            width: 240.0,
            corner_radius: 8.0,
            fill: None,
            border: None,
            title_color: None,
            content_color: None,
            title_size: 16.0,
            content_size: 15.0,
        }
    }

    /// Body text below the title; wraps at the toast width.
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }
    /// Time on screen before the fade-out starts; 4 seconds.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }
    /// Outer width, limited to the viewport; 240.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(80.0);
        self
    }
    pub fn corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius.max(0.0);
        self
    }
    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }
    pub fn border(mut self, color: Color) -> Self {
        self.border = Some(color);
        self
    }
    pub fn title_color(mut self, color: Color) -> Self {
        self.title_color = Some(color);
        self
    }
    pub fn content_color(mut self, color: Color) -> Self {
        self.content_color = Some(color);
        self
    }
    /// Font sizes of the title and the body; 16 and 15.
    pub fn text_sizes(mut self, title: f32, content: f32) -> Self {
        self.title_size = title.clamp(1.0, 256.0);
        self.content_size = content.clamp(1.0, 256.0);
        self
    }
}

impl Context {
    /// Show a [`Toast`]. Several toasts stack upward, the newest at the bottom.
    pub fn toast(&mut self, toast: Toast) {
        self.push_toast(toast);
    }

    /// Number of toasts still queued or fading out.
    pub fn toast_count(&self) -> usize {
        self.toast_len()
    }
}
