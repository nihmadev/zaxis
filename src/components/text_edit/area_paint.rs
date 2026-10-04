//! Painting of a multi-line field inside its scroll area. Only paragraphs that intersect
//! the viewport are laid out, selected and emitted; selection, text and caret are separate
//! cached elements so a blinking caret never rebuilds the text mesh.
use super::{
    area::AreaState,
    blink::caret_visible,
    chrome::{Chrome, Look},
    *,
};

pub(super) struct AreaPaint<'a> {
    pub id: Id,
    pub look: &'a Look,
    pub chrome: &'a Chrome,
    /// Viewport of this pass, where content is clipped.
    pub view: Rect,
    /// Measured content size reported to the scroll area.
    pub content: Vec2,
    pub focused: bool,
    pub activity: bool,
    pub composing: Option<&'a (String, Option<(usize, usize)>)>,
}

impl TextEdit<'_> {
    pub(super) fn paint_area(
        &self,
        child: &mut Ui<'_>,
        p: &AreaPaint<'_>,
        state: &mut TextEditState,
        area: &mut AreaState,
    ) {
        let (style, component) = (&p.look.style, &p.look.component);
        let origin = child.layout.bounds.min;
        child.layout.used = p.content;
        let text: &str = self.text;
        let doc = &mut area.doc;
        let env = doc.env();
        let clip = child.clip.intersect(p.view);
        let (top, bottom) = (p.view.min.y - origin.y, p.view.max.y - origin.y);
        let first = doc.para_at_y(top);
        let last = doc.measure_range(child.context, text, first, bottom);
        if (doc.height() - p.content.y).abs() > 0.5 {
            // Lines were measured after the scroll area learned the height: settle next pass.
            child.context.request_repaint();
        }
        let cursor_width = style.text_edit_cursor_width;
        let selection = state.buffer.selection();
        let selection_fill = self.selection_fill(p.look);
        let color = p.chrome.color;
        let (window, id) = (child.window, p.id);
        let selecting = p.focused && p.composing.is_none() && !selection.is_empty();

        let mut shapes = Vec::new();
        let mut rects = Vec::new();
        if selecting {
            rects = doc.selection_rects(
                child.context,
                text,
                selection.clone(),
                first..last + 1,
                env.size * 0.4,
            );
            for rect in &rects {
                shapes.push(Paint::Shape(
                    Shape::rect(rect.translate(origin), selection_fill).into(),
                ));
            }
        }

        // The composition string: underline, and the active clause highlighted.
        let cursor = state.buffer.cursor;
        let mut underline = Vec::new();
        if let Some((shown, clause)) = p.composing {
            let para = doc.para_at(cursor);
            let rel = cursor - doc.paras[para].start;
            let layout = doc.layout(child.context, text, para);
            let base = origin + Vec2::new(0.0, doc.top(para));
            for l in &layout.lines {
                for (x0, x1) in l.spans(rel..rel + shown.len()) {
                    let y = base.y + l.top + l.height - cursor_width;
                    underline.push(line(
                        Vec2::new(origin.x + x0, y),
                        Vec2::new(origin.x + x1, y),
                        cursor_width,
                        color,
                    ));
                }
                if let Some((a, b)) = *clause {
                    for (x0, x1) in l.spans(rel + a.min(shown.len())..rel + b.min(shown.len())) {
                        let rect = Rect::from_min_size(
                            Vec2::new(origin.x + x0, base.y + l.top),
                            Vec2::new(x1 - x0, l.height),
                        );
                        shapes.push(Paint::Shape(Shape::rect(rect, selection_fill).into()));
                    }
                }
            }
        }

        let paragraph = |doc: &doc::Doc, i: usize, y: f32, color: Color| Paint::Paragraph {
            text: doc.shown(text, i).to_owned(),
            position: origin + Vec2::new(0.0, y),
            size: env.size,
            weight: env.weight,
            wrap_width: env.wrap,
            tab: env.tab,
            color,
        };
        let mut texts = Vec::new();
        for i in first..=last {
            if !doc.shown(text, i).is_empty() {
                let y = doc.top(i);
                texts.push(paragraph(doc, i, y, color));
            }
        }
        let placeholder = text.is_empty() && p.composing.is_none();
        if placeholder && !self.placeholder.is_empty() {
            texts.push(Paint::Paragraph {
                text: self.placeholder.clone(),
                position: origin,
                size: env.size,
                weight: env.weight,
                wrap_width: env.wrap,
                tab: env.tab,
                color: self.placeholder_fill(p.look),
            });
        }
        shapes.extend(underline);
        child.context.paint(id.with("selection"), window, clip, shapes);
        child.context.paint(id.with("text"), window, clip, texts);
        if let Some(foreground) = component.selection_foreground.filter(|_| selecting) {
            for (k, rect) in rects.iter().enumerate() {
                let i = doc.para_at_y(rect.min.y + 1.0);
                let y = doc.top(i);
                let selected = vec![paragraph(doc, i, y, foreground)];
                let region = clip.intersect(rect.translate(origin));
                child
                    .context
                    .paint(id.with(("selected", k)), window, region, selected);
            }
        }

        let mut caret = Vec::new();
        if p.focused {
            let loc = match p.composing {
                Some((shown, clause)) => {
                    let para = doc.para_at(cursor);
                    let rel = cursor - doc.paras[para].start;
                    let inside = clause.map_or(shown.len(), |(start, _)| start.min(shown.len()));
                    doc.locate_in(child.context, text, para, rel + inside, false)
                }
                None => doc.locate(child.context, text, state.buffer.pos()),
            };
            let at = origin + Vec2::new(loc.x, loc.y);
            let show = caret_visible(
                child,
                id,
                state,
                style.text_edit_blink_interval,
                p.activity,
                p.composing.is_some() || clip.is_empty(),
            );
            if show && p.composing.is_none_or(|(_, clause)| clause.is_some()) {
                caret.push(line(
                    at,
                    at + Vec2::new(0.0, env.lh),
                    cursor_width,
                    p.chrome.caret,
                ));
            }
            if !self.read_only && !clip.is_empty() {
                let x = at.x.clamp(p.view.min.x, p.view.max.x);
                child.context.set_ime_area(
                    window,
                    Rect::from_min_size(Vec2::new(x, at.y), Vec2::new(cursor_width, env.lh))
                        .intersect(clip),
                );
            }
        }
        child.context.paint(id.with("caret"), window, clip, caret);
    }
}
