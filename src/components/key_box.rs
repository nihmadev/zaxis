use std::{hash::Hash, time::Duration};

use winit::keyboard::KeyCode;

use super::{font_size, visible_label, Response, Sense, Ui, Widget};
use crate::{
    context::Paint, Border, Color, Context, Easing, Id, Rect, Repeat, Shape, Tween, TweenOptions,
    Vec2,
};

#[doc(hidden)]
pub mod binding;
pub use binding::{KeyBinding, MouseBinding};

/// Which key box is waiting for input. Only one captures at a time; the pass number
/// lets a capture whose box stopped being built (another tab, a closed window) lapse.
#[derive(Default)]
pub(crate) struct KeyCapture {
    id: Option<Id>,
    frame: u64,
    middle_down: bool,
}

impl Context {
    /// Whether a [`KeyBox`] is waiting for a key or button. Hotkeys that would otherwise
    /// fire, such as the one that hides a menu, should ignore input meanwhile.
    pub fn key_capture_active(&self) -> bool {
        self.key_capture.id.is_some() && self.key_capture.frame + 1 >= self.frame
    }
}

/// A labeled box that records the next key or mouse button as a [`KeyBinding`].
///
/// Click it to start listening; it reads `...` and pulses. The next key press binds,
/// Escape or a press elsewhere cancels, Backspace or Delete unbinds, and the secondary
/// and middle mouse buttons bind as `RMB` and `MMB`. The primary button binds only with
/// [`KeyBox::allow_left_click`]. `changed()` is set on the pass that changes the binding.
///
/// ```no_run
/// # fn menu(ui: &mut zaxis::Ui<'_>) {
/// let mut aim = zaxis::KeyBinding::Key(zaxis::winit::keyboard::KeyCode::KeyE);
/// ui.add(zaxis::KeyBox::new(&mut aim, "aim key"));
/// # }
/// ```
pub struct KeyBox<'a> {
    binding: &'a mut KeyBinding,
    label: String,
    id: Option<Id>,
    enabled: bool,
    allow_left: bool,
    button: Vec2,
    fill: Option<Color>,
    active_fill: Option<Color>,
    accent: Option<Color>,
}

impl<'a> KeyBox<'a> {
    pub fn new(binding: &'a mut KeyBinding, label: impl Into<String>) -> Self {
        Self {
            binding,
            label: label.into(),
            id: None,
            enabled: true,
            allow_left: false,
            button: Vec2::new(104.0, 24.0),
            fill: None,
            active_fill: None,
            accent: None,
        }
    }

    pub fn id_source(mut self, source: impl Hash) -> Self {
        self.id = Some(Id::new(source));
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Let a primary-button press outside the box bind `LMB` instead of cancelling.
    pub fn allow_left_click(mut self, allow: bool) -> Self {
        self.allow_left = allow;
        self
    }
    /// Size of the key button at the right edge; 104 × 24.
    pub fn button_size(mut self, size: Vec2) -> Self {
        self.button = size.max(Vec2::splat(8.0));
        self
    }
    /// Idle fill of the button; `Style::button_fill`.
    pub fn fill(mut self, color: Color) -> Self {
        self.fill = Some(color);
        self
    }
    /// Fill while hovered or listening; `Style::selected_fill`.
    pub fn active_fill(mut self, color: Color) -> Self {
        self.active_fill = Some(color);
        self
    }
    /// Text and outline color while listening; `Style::accent`.
    pub fn accent(mut self, color: Color) -> Self {
        self.accent = Some(color);
        self
    }

    /// The binding the input of this pass selects, if any. Escape and a press elsewhere
    /// select the current binding again, which ends the capture without a change.
    fn capture(&self, context: &mut Context, over_button: bool) -> Option<KeyBinding> {
        let middle_down = context.input().middle_down;
        let middle_edge = middle_down && !context.key_capture.middle_down;
        context.key_capture.middle_down = middle_down;
        let input = context.input();
        if input.keys_pressed.contains(&KeyCode::Escape) {
            return Some(*self.binding);
        }
        if input.keys_pressed.contains(&KeyCode::Backspace)
            || input.keys_pressed.contains(&KeyCode::Delete)
        {
            return Some(KeyBinding::None);
        }
        if let Some(code) = input
            .keys_pressed
            .iter()
            .min_by_key(|code| format!("{code:?}"))
        {
            return Some(KeyBinding::Key(*code));
        }
        if input.secondary_pressed {
            return Some(KeyBinding::Mouse(MouseBinding::Right));
        }
        if middle_edge {
            return Some(KeyBinding::Mouse(MouseBinding::Middle));
        }
        if input.primary_pressed && !over_button {
            return Some(if self.allow_left {
                KeyBinding::Mouse(MouseBinding::Left)
            } else {
                *self.binding
            });
        }
        None
    }
}

impl Widget for KeyBox<'_> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let enabled = self.enabled && ui.is_enabled();
        let style = ui.style().clone();
        let size = font_size(style.font_size);
        let weight = style.typography.weights.control;
        let row = ui.allocate_space(Vec2::new(ui.available_width(), 28.0_f32.max(self.button.y)));
        let button = Rect::from_min_size(
            Vec2::new(
                row.max.x - self.button.x.min(row.size().x),
                row.center().y - self.button.y * 0.5,
            ),
            self.button,
        );
        let key = self.id.unwrap_or_else(|| Id::new(&self.label));
        let mut response = ui.interact(button, ("key-box", key), Sense::CLICK | Sense::FOCUS);
        let id = response.id;
        let mut capturing = ui.context.key_capture.id == Some(id);
        if capturing && !enabled {
            ui.context.key_capture = KeyCapture::default();
            capturing = false;
        }
        if capturing {
            ui.context.key_capture.frame = ui.context.frame;
        }
        if enabled && response.clicked() {
            if capturing {
                ui.context.key_capture = KeyCapture::default();
            } else {
                ui.context.key_capture = KeyCapture {
                    id: Some(id),
                    frame: ui.context.frame,
                    middle_down: ui.context.input().middle_down,
                };
            }
            capturing = !capturing;
        } else if capturing {
            if let Some(next) = self.capture(ui.context, response.hovered) {
                if *self.binding != next {
                    *self.binding = next;
                    response.changed = true;
                }
                ui.context.key_capture = KeyCapture::default();
                capturing = false;
            }
        }
        // A button that is pressed while it listens; its value is the binding it shows.
        ui.a11y(id, row, crate::AccessRole::Button, |node| {
            node.label(visible_label(&self.label))
                .value(self.binding.label())
                .toggled(capturing)
                .disabled(!enabled)
                .clicks(id);
        });
        let hovered = enabled && response.hovered;
        let engaged = capturing || hovered;
        let ease = || TweenOptions::new(Duration::from_millis(160)).easing(Easing::QuadOut);
        let scale = if capturing {
            ui.animate(("pulse", key), || {
                Tween::new(1.0_f32, 1.035, Duration::from_millis(650))
                    .easing(Easing::SineInOut)
                    .repeat(Repeat::Forever)
                    .auto_reverse(true)
            })
            .value
        } else {
            ui.transition(
                ("scale", key),
                if hovered { 1.025 } else { 1.0_f32 },
                ease(),
            )
            .value
        };
        let accent = self.accent.unwrap_or(style.accent);
        let fill = if engaged {
            self.active_fill.unwrap_or(style.selected_fill)
        } else {
            self.fill.unwrap_or(style.button_fill)
        };
        let fill = ui.transition(("fill", key), fill, ease()).value;
        let text_color = if !enabled {
            style.disabled_text
        } else if capturing {
            accent
        } else if hovered {
            style.text_color
        } else {
            style.muted_text
        };
        let text_color = ui.transition(("text", key), text_color, ease()).value;
        let outline_target = if capturing {
            0.75
        } else if hovered {
            0.35
        } else {
            0.0_f32
        };
        let outline = ui
            .transition(("outline", key), outline_target, ease())
            .value;
        let label_target = if !enabled {
            style.disabled_text
        } else if engaged {
            style.text_color
        } else {
            style.muted_text
        };
        let label_color = ui.transition(("label", key), label_target, ease()).value;
        let shown = Rect::from_min_size(
            button.center() - button.size() * 0.5 * scale,
            button.size() * scale,
        );
        let mut shapes = vec![Paint::Shape(
            Shape::rect(shown, fill).corner_radius(5.0).into(),
        )];
        if outline > 0.0 || response.focus_visible {
            let border = if outline > 0.0 {
                Border::new(1.0, accent.with_opacity(outline))
            } else {
                style.focus_border
            };
            shapes.push(Paint::Shape(
                Shape::rect(shown, Color::TRANSPARENT)
                    .corner_radius(5.0)
                    .border(border)
                    .into(),
            ));
        }
        ui.context
            .paint(id.with("button"), ui.window, ui.clip, shapes);
        let caption = if capturing {
            "...".to_owned()
        } else {
            self.binding.label()
        };
        let small = (size - 3.0).max(1.0) * scale;
        let caption_size = ui
            .context
            .measure_text(&caption, small, weight, f32::INFINITY);
        let label = visible_label(&self.label).to_owned();
        let label_size = ui.context.measure_text(&label, size, weight, f32::INFINITY);
        ui.context.paint(
            id.with("text"),
            ui.window,
            ui.clip,
            vec![
                Paint::Text {
                    text: caption,
                    position: shown.center() - caption_size * 0.5,
                    size: small,
                    weight,
                    wrap_width: f32::INFINITY,
                    color: text_color,
                },
                Paint::Text {
                    text: label,
                    position: Vec2::new(row.min.x, row.center().y - label_size.y * 0.5),
                    size,
                    weight,
                    wrap_width: (button.min.x - row.min.x).max(0.0),
                    color: label_color,
                },
            ],
        );
        response
    }
}

impl Ui<'_> {
    /// Shorthand for `ui.add(KeyBox::new(binding, label))`.
    pub fn key_box(&mut self, binding: &mut KeyBinding, label: impl Into<String>) -> Response {
        self.add(KeyBox::new(binding, label))
    }
}
