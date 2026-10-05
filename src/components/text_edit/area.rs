//! The multi-line field: height resolution, event handling against the paragraph table,
//! scroll commands (anchoring, paging, caret reveal, drag auto-scroll) and hand-off to
//! the standard `ScrollArea`, which owns the offset, wheel routing and scroll bars.
use super::{
    chrome::Look,
    doc::{Doc, Env},
    events::{Geo, Outcome},
    scroll::{Anchor, DragScroll},
    text_input::Fingerprint,
    *,
};
use crate::{context::Context, ScrollArea};

pub(crate) struct AreaState {
    pub doc: Doc,
    /// Viewport of the last pass in the field's own coordinates; pointer events and
    /// page sizes refer to the geometry the user saw.
    view: Rect,
    drag: DragScroll,
    /// The published lines and what they were built from; see [`super::access`].
    pub access: Option<(super::access::AreaKey, crate::accessibility::text::TextRef)>,
}

impl AreaState {
    fn new(text: &str, env: Env) -> Self {
        Self {
            doc: Doc::new(text, env),
            view: Rect::default(),
            drag: DragScroll::new(),
            access: None,
        }
    }
}

/// Vertical offset that brings `target` (content space) into a viewport of height `h`.
fn reveal_y(offset: f32, h: f32, target: Rect) -> f32 {
    if target.min.y < offset || target.size().y > h {
        target.min.y
    } else if target.max.y > offset + h {
        target.max.y - h
    } else {
        offset
    }
}

impl TextEdit<'_> {
    fn area_height(&self, look: &Look, lh: f32, doc: &mut Doc, ctx: &mut Context) -> f32 {
        let pad = look.padding.size().y;
        let min_token = look.component.area_min_height.unwrap_or(64.0).max(lh + pad);
        if let Some(height) = self.height {
            return height.max(lh + pad);
        }
        match self.area.map(|a| a.height) {
            Some(AreaHeight::Rows(rows)) => rows * lh + pad,
            Some(AreaHeight::Auto { min, max }) => {
                let min_h = min.map_or(min_token, |rows| rows * lh + pad);
                let max_h = max.map_or(f32::INFINITY, |rows| rows * lh + pad).max(min_h);
                // Only the paragraphs that can matter are shaped: everything past the
                // maximum height is clamped away.
                doc.measure_range(ctx, self.text, 0, max_h - pad);
                (doc.height() + pad).clamp(min_h, max_h)
            }
            _ => min_token,
        }
    }

    pub(super) fn show_area(mut self, ui: &mut Ui<'_>, id: Id, look: Look) -> Response {
        let opts = self.area.expect("multi-line options");
        let (size, font, padding) = (look.size, look.font, look.padding);
        let style = &look.style;
        let lh = ui.context.measure_text("", size, font, f32::INFINITY).y;
        let available = ui.available_width();
        let width = match self.width {
            Some(width) => width,
            None if opts.fill_width => available,
            None => ui.layout.preferred_width.unwrap_or(available),
        }
        .clamp(0.0, available);
        let caret_width = style.text_edit_cursor_width;
        let env = Env {
            size,
            font,
            wrap: if opts.wrap {
                (width - padding.size().x - caret_width).max(1.0)
            } else {
                f32::INFINITY
            },
            tab: opts.tab_size,
            lh,
        };
        let mut state = ui
            .context
            .text_edits
            .remove(&id)
            .unwrap_or_else(|| TextEditState::new(self.text, style.text_edit_blink_interval));
        let mut area = state
            .area
            .take()
            .unwrap_or_else(|| Box::new(AreaState::new(self.text, env)));
        area.doc.frame = ui.context.frame;
        area.doc.clear_overlay();
        let scroll_id = ScrollArea::state_id(ui, id.with("scroll"));
        let offset = ui
            .context
            .scrolling
            .states
            .get(&scroll_id)
            .map_or(Vec2::ZERO, |s| s.offset);
        // Remember the top line before anything can move it.
        let mut anchor = Anchor::capture(&mut area.doc, ui.context, self.text, offset.y);
        let mut fingerprint = Fingerprint::of(self.text);
        area.doc.set_env(env);
        if state.fingerprint != fingerprint {
            state.text_replaced(self.text);
            area.doc.reset(self.text);
        }

        let view = area.view;
        let axes = [!opts.wrap, true];
        let mut command: Option<Vec2> = None;
        let mut current = offset;
        let mut drag_pointer = None;
        if let Some((delta, pointer)) = area.drag.step(ui.context, id, view, axes) {
            current = (offset + delta).max(Vec2::ZERO);
            command = Some(current);
            drag_pointer = Some(pointer);
        }
        let mut events = ui.context.take_text_edit_input(id);
        super::access::requests(ui.context, id, &mut events);
        let activity_in = !events.is_empty();
        let focused = ui.context.has_focus(id) && self.enabled;
        let mut out = Outcome::default();
        let page = ((view.size().y / lh).floor() as i32 - 1).max(1);
        let mut geo = Geo {
            doc: Some(&mut area.doc),
            size,
            font,
            origin: view.min - current,
            page,
        };
        self.process_events(ui, &mut state, &mut out, events, &mut geo);
        if let Some(pointer) = drag_pointer {
            self.pointer(ui, &mut state, &mut geo, pointer, true, 0);
        }
        for delta in out.deltas.drain(..) {
            anchor.shift(delta);
        }
        if out.scroll_lines != 0 {
            current.y = (current.y + out.scroll_lines as f32 * lh).max(0.0);
            command = Some(current);
        }
        if !focused || self.read_only {
            state.preedit = None;
        }
        let doc = &mut area.doc;
        let composing = state.preedit.clone();
        let cursor = state.buffer.cursor;
        if let Some((text, _)) = &composing {
            let para = doc.para_at(cursor);
            doc.compose(
                ui.context,
                self.text,
                para,
                cursor - doc.paras[para].start,
                text,
            );
        }
        if out.changed {
            fingerprint = Fingerprint::of(self.text);
        }

        let height = self.area_height(&look, lh, doc, ui.context);
        let view_h = (height - padding.size().y).max(0.0);
        if command.is_none() {
            let y = anchor.resolve(doc, ui.context, self.text);
            if (y - offset.y).abs() > 0.5 {
                command = Some(Vec2::new(current.x, y.max(0.0)));
            }
        }
        // Settle heights around the offset the scroll area will end up with, so what is
        // measured is what gets painted; the caret is revealed on the same terms.
        let mut target = None;
        let mut y = command.map_or(offset.y, |c| c.y);
        for _ in 0..3 {
            let first = doc.para_at_y(y.max(0.0));
            doc.measure_range(ui.context, self.text, first, y + view_h);
            if out.reveal && focused {
                let pos = state.buffer.pos();
                let loc = match &composing {
                    Some((text, caret)) => {
                        let para = doc.para_at(cursor);
                        let rel = cursor - doc.paras[para].start;
                        let inside = caret.map_or(text.len(), |(start, _)| start.min(text.len()));
                        doc.locate_in(ui.context, self.text, para, rel + inside, false)
                    }
                    None => doc.locate(ui.context, self.text, pos),
                };
                target = Some(Rect::from_min_size(
                    Vec2::new(loc.x - 2.0, loc.y),
                    Vec2::new(caret_width + 4.0, lh),
                ));
            }
            let mut next = target.map_or(y, |t| reveal_y(y, view_h, t));
            next = next.clamp(0.0, (doc.height() - view_h).max(0.0));
            let settled = (next - y).abs() < 0.5;
            y = next;
            if settled {
                break;
            }
        }
        let content = Vec2::new(
            if opts.wrap {
                0.0
            } else {
                doc.width() + caret_width + 2.0
            },
            doc.height(),
        );

        let rect = ui.allocate_space(Vec2::new(width, height));
        let view = padding.inset(rect);
        let mut response = ui.response(id, rect, self.enabled);
        let activity = activity_in || state.focused != response.has_focus;
        response.lost_focus = (state.focused && !response.has_focus) || out.lost_focus;
        response.submitted |= out.submitted;
        response.changed = out.changed;
        let chrome = self.paint_chrome(ui, id, rect, response, &look);
        if opts.tab_indent && self.enabled && !self.read_only {
            ui.context.text_edit_tabs.insert(id);
        }

        let mut scroll = if opts.wrap {
            ScrollArea::vertical()
        } else {
            ScrollArea::both()
        }
        .id(id.with("scroll"))
        .style(crate::ScrollStyle {
            padding,
            ..style.scroll
        })
        .overlay_scrollbars(true)
        // The field itself is the node; its scroll area is an implementation detail.
        .a11y_hidden();
        if let Some(command) = command.filter(|c| c.is_finite()) {
            scroll = scroll.scroll_offset(command);
        }
        if let Some(target) = target {
            scroll = scroll.scroll_to_rect(target);
        }
        let paint = area_paint::AreaPaint {
            id,
            rect,
            look: &look,
            chrome: &chrome,
            view,
            content,
            focused: response.has_focus,
            activity,
            composing: composing.as_ref(),
        };
        let height_before = content.y;
        let lines = scroll
            .show_at(ui, rect, Some(height_before), |child| {
                self.paint_area(child, &paint, &mut state, &mut area)
            })
            .inner;
        if lines.is_some() {
            let shown = composing.as_ref().map(|_| {
                let para = area.doc.para_at(cursor);
                area.doc.shown(self.text, para).to_owned()
            });
            self.describe(ui, id, rect, lines, shown.as_deref());
        }

        area.view = view;
        area.doc.trim();
        state.area = Some(area);
        state.focused = response.has_focus;
        state.blink_interval = style.text_edit_blink_interval;
        state.fingerprint = fingerprint;
        ui.context.text_edits.insert(id, state);
        response
    }
}
