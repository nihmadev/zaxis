use std::time::Instant;

use super::{Context, Id, Paint};
use crate::{Border, Color, Rect, Shape, TooltipStyle, Vec2};

pub(crate) struct TooltipRequest {
    pub target: Id,
    pub anchor: Id,
    pub text: String,
    pub style: TooltipStyle,
    pub font_size: f32,
    pub fill: Color,
    pub color: Color,
    pub border: Border,
}

#[derive(Default)]
pub(crate) struct Tooltips {
    pub pending: Vec<TooltipRequest>,
    hovered: Option<(Id, Instant)>,
}

impl Context {
    pub(super) fn finish_tooltips(&mut self) {
        let requests = std::mem::take(&mut self.tooltips.pending);
        let mut candidate = None;
        for request in requests {
            let marker = self
                .hits
                .iter()
                .find(|hit| hit.id == request.anchor)
                .copied();
            // Real widget hits have the final geometry even if show() was called
            // after the widget's individual layout capture finished.
            let hit = self
                .hits
                .iter()
                .find(|hit| hit.id == request.target)
                .copied()
                .or(marker);
            self.hits.retain(|hit| hit.id != request.anchor);
            let Some(hit) = hit else { continue };
            if self.input.focused
                && !self.input.primary_down
                && !self.input.middle_down
                && self.input.pointer.is_some_and(|p| {
                    hit.rect.intersect(hit.clip).contains(p)
                        && self.top_window(p).is_none_or(|top| top == hit.window)
                })
            {
                candidate = Some((request, hit.rect.intersect(hit.clip)));
            }
        }
        let Some((request, anchor)) = candidate else {
            self.tooltips.hovered = None;
            return;
        };
        let start = match self.tooltips.hovered {
            Some((id, start)) if id == request.target => start,
            _ => self.frame_time,
        };
        self.tooltips.hovered = Some((request.target, start));
        let elapsed = self.frame_time.saturating_duration_since(start);
        if elapsed < request.style.delay {
            self.request_repaint_after(request.style.delay - elapsed);
            return;
        }
        let viewport = self.viewport();
        let padding = request.style.padding;
        let inset = Vec2::new(padding.left + padding.right, padding.top + padding.bottom);
        let available = (viewport.size() - inset - Vec2::splat(8.0)).max(Vec2::splat(1.0));
        let mut width = request
            .style
            .max_width
            .max(request.font_size * 8.0)
            .min(available.x);
        let (mut text, mut size, mut wrap) =
            self.tooltip_text(&request.text, request.font_size, width);
        // Prefer a wider box to losing lines when the viewport is short.
        if size.y > available.y && width < available.x {
            width = available.x;
            (text, size, wrap) = self.tooltip_text(&request.text, request.font_size, width);
        }
        let size = size + inset;
        let rect = place(anchor, size, viewport);
        let id = request.target.with("tooltip");
        self.popup_layers.push(id);
        self.paint(
            id,
            id,
            viewport,
            vec![
                Paint::Shape(
                    Shape::rect(rect, request.fill)
                        .corner_radius(request.style.rounding)
                        .border(request.border)
                        .into(),
                ),
                Paint::Text {
                    text,
                    position: rect.min + Vec2::new(padding.left, padding.top),
                    size: request.font_size,
                    wrap_width: wrap,
                    color: request.color,
                },
            ],
        );
    }

    fn tooltip_text(&mut self, text: &str, font: f32, width: f32) -> (String, Vec2, f32) {
        // Wrap at words first. The shared shaper handles oversized words and
        // grapheme clusters, using exactly the same width for measurement/paint.
        let mut output = String::new();
        for (n, paragraph) in text.split('\n').enumerate() {
            if n > 0 {
                output.push('\n');
            }
            let mut line = String::new();
            for word in paragraph.split_whitespace() {
                let next = if line.is_empty() {
                    word.to_owned()
                } else {
                    format!("{line} {word}")
                };
                if !line.is_empty() && self.measure_text(&next, font, f32::INFINITY).x > width {
                    output.push_str(&line);
                    output.push('\n');
                    line = word.to_owned();
                } else {
                    line = next;
                }
            }
            output.push_str(&line);
        }
        let (size, wrap) = self.measure_text_layout(&output, font, width);
        (output, size, wrap)
    }
}

fn place(anchor: Rect, size: Vec2, viewport: Rect) -> Rect {
    let below = anchor.max.y + 6.0;
    let above = anchor.min.y - 6.0 - size.y;
    let y = if below + size.y <= viewport.max.y {
        below
    } else {
        above
    };
    let max = (viewport.max - size - Vec2::splat(4.0)).max(viewport.min);
    let min = Vec2::new(anchor.min.x, y)
        .clamp(viewport.min + Vec2::splat(4.0).min(max - viewport.min), max);
    Rect::from_min_size(min, size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vec2, Align, Button, Padding, Root, Tooltip};
    use std::time::Duration;
    use winit::{dpi::PhysicalSize, event::ElementState};

    fn setup() -> Context {
        let mut c = Context::new();
        c.set_viewport(PhysicalSize::new(640, 360), 1.0);
        c
    }

    fn draw(c: &mut Context, now: Instant, enabled: bool) -> Id {
        let mut target = Id::new("unused");
        c.run_at(now, |c| {
            Root::new().show(c, |ui| {
                ui.horizontal_aligned(Align::Center, |ui| {
                    ui.spacer();
                    target = ui
                        .add(
                            Tooltip::new(
                                "A useful explanation that is much wider than this tiny button.",
                            )
                            .enabled(enabled)
                            .wrap(Button::new("?")),
                        )
                        .id;
                });
            });
        });
        target
    }

    #[test]
    fn tooltip_delay_layout_and_passive_input() {
        let mut c = setup();
        let now = Instant::now();
        let target = draw(&mut c, now, true);
        let hit = *c.previous_hits.iter().find(|h| h.id == target).unwrap();
        c.move_pointer(hit.rect.center());
        draw(&mut c, now, true);
        assert!(!c.seen.contains(&target.with("tooltip")));
        assert!(c.next_repaint().is_some());
        let hits = c.previous_hits.len();
        let later = now + Duration::from_millis(400);
        draw(&mut c, later, true);
        let paint = &c.cache[&target.with("tooltip")].paint;
        let Paint::Shape(Shape::Rect { rect, .. }) = &paint[0] else {
            panic!("tooltip box")
        };
        assert!(rect.size().x > hit.rect.size().x * 3.0);
        assert!(rect.min.x >= c.viewport().min.x && rect.max.x <= c.viewport().max.x);
        assert_eq!(c.previous_hits.len(), hits);
        assert!(c.popup.is_none());
        assert!(c.focused_widget.is_none());
        assert_eq!(c.hit_test(hit.rect.center()).unwrap().id, target);
        c.primary_button(ElementState::Pressed);
        draw(&mut c, later, true);
        assert!(!c.seen.contains(&target.with("tooltip")));
        c.primary_button(ElementState::Released);
        draw(&mut c, later, false);
        assert!(!c.seen.contains(&target.with("tooltip")));
    }

    #[test]
    fn text_wraps_at_words_and_fits_above_bottom_anchor() {
        let mut c = setup();
        let (text, size, wrap) = c.tooltip_text(
            "Первая строка с пояснением\nSecond paragraph with words",
            14.0,
            180.0,
        );
        assert!(text.contains("\nSecond"));
        assert!(size.x <= 180.0 && size.y < 140.0);
        assert_eq!(c.measure_text(&text, 14.0, wrap), size);
        let anchor = Rect::from_min_size(vec2(610.0, 330.0), vec2(20.0, 20.0));
        let rect = place(anchor, size + vec2(20.0, 12.0), c.viewport());
        assert!(rect.max.y < anchor.min.y);
        assert!(rect.max.x <= 640.0 && rect.min.x >= 0.0);
    }

    #[test]
    fn passive_text_can_have_a_tooltip_and_global_disable_is_respected() {
        let mut c = setup();
        c.style.tooltip.delay = Duration::ZERO;
        c.move_pointer(vec2(25.0, 25.0));
        let mut id = Id::new("unused");
        let build = |c: &mut Context, id: &mut Id| {
            Root::new().padding(Padding::all(20.0)).show(c, |ui| {
                *id = ui
                    .add(Tooltip::new("Label explanation").wrap(crate::Text::new("Label")))
                    .id;
            });
        };
        c.run(|c| build(c, &mut id));
        assert!(c.seen.contains(&id.with("tooltip")));
        assert!(!c
            .previous_hits
            .iter()
            .any(|h| h.id == id.with("tooltip-anchor")));
        c.style.tooltip.enabled = false;
        c.run(|c| build(c, &mut id));
        assert!(!c.seen.contains(&id.with("tooltip")));
    }

    #[test]
    fn tooltip_follows_scrolling_and_escapes_the_scroll_clip() {
        let mut c = setup();
        c.style.tooltip.delay = Duration::ZERO;
        let mut target = Id::new("unused");
        let build = |c: &mut Context, target: &mut Id, offset: f32| {
            Root::new().show(c, |ui| {
                crate::ScrollArea::vertical()
                    .max_height(60.0)
                    .scroll_offset(vec2(0.0, offset))
                    .show(ui, |ui| {
                        ui.add_space(45.0);
                        let response = ui.button("Target");
                        *target = response.id;
                        ui.tooltip(response, "A hint outside the small scrolling viewport.");
                        ui.add_space(150.0);
                    });
            });
        };
        c.run(|c| build(c, &mut target, 30.0));
        let hit = *c.previous_hits.iter().find(|h| h.id == target).unwrap();
        c.move_pointer(hit.rect.intersect(hit.clip).center());
        c.run(|c| build(c, &mut target, 30.0));
        let Paint::Shape(Shape::Rect { rect, .. }) = c.cache[&target.with("tooltip")].paint[0]
        else {
            panic!("tooltip box");
        };
        assert!(rect.min.y >= hit.rect.max.y);
        assert!(rect.max.y > hit.clip.max.y);
        c.run(|c| build(c, &mut target, 120.0));
        assert!(!c.seen.contains(&target.with("tooltip")));
    }

    #[test]
    fn response_hint_follows_a_centered_passive_label() {
        let mut c = setup();
        c.style.tooltip.delay = Duration::ZERO;
        let mut target = Id::new("unused");
        c.move_pointer(vec2(320.0, 25.0));
        c.run(|c| {
            Root::new().padding(Padding::all(20.0)).show(c, |ui| {
                ui.vertical_aligned(Align::Center, |ui| {
                    let response = ui.label("Centered label");
                    target = response.id;
                    ui.tooltip(response, "Explanation");
                });
            });
        });
        assert!(c.seen.contains(&target.with("tooltip")));
        let Paint::Shape(Shape::Rect { rect, .. }) = c.cache[&target.with("tooltip")].paint[0]
        else {
            panic!("tooltip box");
        };
        assert!(rect.min.x > 200.0);
    }
}
