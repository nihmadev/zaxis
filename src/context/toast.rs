use crate::time::Instant;
use std::time::Duration;

use super::{Context, Id, Paint};
use crate::{Border, Color, Rect, Shape, Toast, Vec2};

const MARGIN: f32 = 20.0;
const GAP: f32 = 8.0;
const PADDING: Vec2 = Vec2::new(12.0, 8.0);
const ENTER: Duration = Duration::from_millis(220);
const EXIT: Duration = Duration::from_millis(350);
const SLIDE: f32 = 16.0;

struct Entry {
    serial: u64,
    toast: Toast,
    born: Instant,
}

#[derive(Default)]
pub(crate) struct Toasts {
    entries: Vec<Entry>,
    serial: u64,
}

fn eased(t: f32) -> f32 {
    1.0 - (1.0 - t.clamp(0.0, 1.0)).powi(2)
}

impl Context {
    pub(crate) fn push_toast(&mut self, toast: Toast) {
        self.toasts.serial += 1;
        let serial = self.toasts.serial;
        self.toasts.entries.push(Entry {
            serial,
            toast,
            born: self.frame_time,
        });
        self.request_repaint();
    }

    pub(crate) fn toast_len(&self) -> usize {
        self.toasts.entries.len()
    }

    /// Paint the queue as the topmost passive layer. Schedules the passes the fades need
    /// and nothing else: a toast that is fully shown only asks for the pass that ends it.
    pub(super) fn finish_toasts(&mut self) {
        let now = self.frame_time;
        self.toasts.entries.retain(|entry| {
            now.saturating_duration_since(entry.born) < entry.toast.duration + EXIT
        });
        if self.toasts.entries.is_empty() {
            return;
        }
        let reduced = self.style.motion.reduced_motion;
        let viewport = self.viewport();
        let layer = Id::new("toasts");
        self.popups.layers.push(layer);
        let mut bottom = viewport.max.y - MARGIN;
        let mut next_pass: Option<Duration> = None;
        let mut animating = false;
        let entries = std::mem::take(&mut self.toasts.entries);
        // Where each toast comes to rest, newest first, for the accessibility tree.
        let mut settled = Vec::new();
        for entry in entries.iter().rev() {
            let toast = &entry.toast;
            let age = now.saturating_duration_since(entry.born);
            let (mut alpha, offset) = if reduced || age >= ENTER {
                (1.0, 0.0)
            } else {
                animating = true;
                let t = eased(age.as_secs_f32() / ENTER.as_secs_f32());
                (t, SLIDE * (1.0 - t))
            };
            if age >= toast.duration {
                alpha = if reduced {
                    0.0
                } else {
                    animating = true;
                    1.0 - ((age - toast.duration).as_secs_f32() / EXIT.as_secs_f32())
                        .clamp(0.0, 1.0)
                };
            } else {
                let left = toast.duration - age;
                next_pass = Some(next_pass.map_or(left, |wait| wait.min(left)));
            }
            let width = toast
                .width
                .min((viewport.size().x - 2.0 * MARGIN).max(80.0));
            let inner = width - 2.0 * PADDING.x;
            let weight = self.style.typography.weights.control;
            let title = self.measure_text(&toast.title, toast.title_size, weight, inner);
            let content = if toast.content.is_empty() {
                Vec2::ZERO
            } else {
                self.measure_text(&toast.content, toast.content_size, weight, inner)
            };
            let height = PADDING.y * 2.0 + title.y + content.y;
            let rect = Rect::from_min_size(
                Vec2::new(viewport.max.x - MARGIN - width + offset, bottom - height),
                Vec2::new(width, height),
            );
            bottom -= height + GAP;
            if self.a11y_on() {
                settled.push(rect.translate(Vec2::new(-offset, 0.0)));
            }
            let fade = |color: Color| color.with_opacity(alpha);
            let fill = fade(toast.fill.unwrap_or(self.style.window_fill));
            let border = fade(toast.border.unwrap_or(self.style.border.color));
            let title_color = fade(toast.title_color.unwrap_or(self.style.text_color));
            let content_color = fade(toast.content_color.unwrap_or(self.style.muted_text));
            let mut paint = vec![
                Paint::Shape(
                    Shape::rect(rect, fill)
                        .corner_radius(toast.corner_radius)
                        .border(Border::new(1.0, border))
                        .into(),
                ),
                Paint::Text {
                    text: toast.title.clone(),
                    position: rect.min + PADDING,
                    size: toast.title_size,
                    weight,
                    wrap_width: inner,
                    color: title_color,
                },
            ];
            if !toast.content.is_empty() {
                paint.push(Paint::Text {
                    text: toast.content.clone(),
                    position: rect.min + PADDING + Vec2::new(0.0, title.y),
                    size: toast.content_size,
                    weight,
                    wrap_width: inner,
                    color: content_color,
                });
            }
            self.paint(Id::new(("toast", entry.serial)), layer, viewport, paint);
        }
        // One live region per toast, oldest first. It is added once and neither its name
        // nor its bounds follow the fade and the slide, so it is announced once.
        for (entry, rect) in entries.iter().zip(settled.into_iter().rev()) {
            let toast = &entry.toast;
            let id = Id::new(("toast", entry.serial));
            let access = self.a11y_begin_layer(layer, id, crate::AccessRole::Status, |node| {
                match (toast.title.is_empty(), toast.content.is_empty()) {
                    (_, true) => node.label(toast.title.as_str()),
                    (true, false) => node.label(toast.content.as_str()),
                    (false, false) => node.label(format!("{}: {}", toast.title, toast.content)),
                };
                node.live(crate::AccessLive::Polite);
            });
            self.a11y_end(access, Some((rect, viewport)));
        }
        self.toasts.entries = entries;
        if animating {
            self.request_repaint();
        }
        if let Some(wait) = next_pass {
            self.request_repaint_after(wait);
        }
    }
}
