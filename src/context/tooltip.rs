use crate::time::Instant;

use super::{Context, Id, Paint};
use crate::{Border, Color, FontWeight, Rect, Shape, TooltipStyle, Vec2};

pub(crate) struct TooltipRequest {
    pub target: Id,
    pub anchor: Id,
    pub text: String,
    pub style: TooltipStyle,
    pub font_size: f32,
    pub font_weight: FontWeight,
    pub fill: Color,
    pub color: Color,
    pub border: Border,
}

#[derive(Default)]
pub(crate) struct Tooltips {
    pub pending: Vec<TooltipRequest>,
    hovered: Option<(Id, Instant)>,
    /// The widget whose tooltip assistive technology asked to see. It stays until it is
    /// asked to hide, the pointer finds another tooltip, a button goes down or the widget
    /// is gone.
    pub shown: Option<Id>,
}

impl Context {
    pub(super) fn finish_tooltips(&mut self) {
        let requests = std::mem::take(&mut self.tooltips.pending);
        let asked = self.tooltips.shown.filter(|_| !self.input.primary_down);
        // A widget behind a modal is not in the accessibility tree; its tooltip goes too.
        let modal = self.top_modal_id().map(|id| self.layer_rank(id));
        let mut candidate = None;
        let mut requested = None;
        for request in requests {
            let marker = self
                .interaction
                .hits
                .iter()
                .find(|hit| hit.id == request.anchor)
                .copied();
            // Real widget hits have the final geometry even if show() was called
            // after the widget's individual layout capture finished.
            let hit = self
                .interaction
                .hits
                .iter()
                .find(|hit| hit.id == request.target)
                .copied()
                .or(marker);
            self.interaction.hits.retain(|hit| hit.id != request.anchor);
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
            } else if asked == Some(request.target)
                && modal.is_none_or(|rank| self.layer_rank(hit.window) >= rank)
            {
                requested = Some((request, hit.rect.intersect(hit.clip)));
            }
        }
        // The pointer's tooltip wins and appears after its delay; one that was asked for
        // is shown at once.
        let hovering = candidate.is_some();
        let kept = match &candidate {
            Some((request, _)) => asked == Some(request.target),
            None => requested.is_some(),
        };
        if !kept {
            self.tooltips.shown = None;
        }
        let Some((request, anchor)) = candidate.or(requested) else {
            self.tooltips.hovered = None;
            return;
        };
        if hovering {
            let start = match self.tooltips.hovered {
                Some((id, start)) if id == request.target => start,
                _ => self.frame_time,
            };
            self.tooltips.hovered = Some((request.target, start));
            let elapsed = self.frame_time.saturating_duration_since(start);
            if elapsed < request.style.delay && !kept {
                self.request_repaint_after(request.style.delay - elapsed);
                return;
            }
        } else {
            self.tooltips.hovered = None;
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
            self.tooltip_text(&request.text, request.font_size, request.font_weight, width);
        // Prefer a wider box to losing lines when the viewport is short.
        if size.y > available.y && width < available.x {
            width = available.x;
            (text, size, wrap) =
                self.tooltip_text(&request.text, request.font_size, request.font_weight, width);
        }
        let size = size + inset;
        let rect = place(anchor, size, viewport);
        let id = request.target.with("tooltip");
        self.popups.layers.push(id);
        let access = self.a11y_begin_layer(id, id, crate::AccessRole::Tooltip, |node| {
            node.label(request.text.as_str());
        });
        self.a11y_end(access, Some((rect, viewport)));
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
                    weight: request.font_weight,
                    wrap_width: wrap,
                    color: request.color,
                },
            ],
        );
    }

    pub fn tooltip_text(
        &mut self,
        text: &str,
        font: f32,
        weight: FontWeight,
        width: f32,
    ) -> (String, Vec2, f32) {
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
                if !line.is_empty()
                    && self.measure_text(&next, font, weight, f32::INFINITY).x > width
                {
                    output.push_str(&line);
                    output.push('\n');
                    line = word.to_owned();
                } else {
                    line = next;
                }
            }
            output.push_str(&line);
        }
        let (size, wrap) = self.measure_text_layout(&output, font, weight, width);
        (output, size, wrap)
    }
}

pub fn place(anchor: Rect, size: Vec2, viewport: Rect) -> Rect {
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
