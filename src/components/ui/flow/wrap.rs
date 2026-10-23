//! A wrapping scope: items are built once at the full width, then packed into lines and
//! placed through the same deferred placement as every other flow.
use super::{
    pack::{clean, pack},
    *,
};

/// How a wrapping scope arranges its items.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Rule {
    pub width: f32,
    pub gap: Vec2,
    /// Placement of each line within the width.
    pub lines: Align,
    /// Placement of each item within the height of its line.
    pub items: Align,
}

/// What a closed wrapping scope occupies, in the coordinates it was built in.
#[derive(Default)]
pub(crate) struct WrapLayout {
    pub size: Vec2,
    pub lines: Vec<Rect>,
    /// One rectangle per item in declaration order; `add_space` is not an item.
    pub items: Vec<Rect>,
}

impl Ui<'_> {
    /// Make this scope a wrapping one. Its items are measured against `rule.width`.
    pub(crate) fn begin_wrap(&mut self, rule: Rule) {
        self.begin_layout(Align::Start);
        if let Some(flow) = &mut self.flow {
            flow.wrap = Some(rule);
        }
        self.layout.cursor = self.layout.bounds.min;
    }

    pub(super) fn report_wrap_spacer(&mut self) {
        let id = self.scope;
        self.context
            .report(crate::DiagnosticKind::InvalidUsage, Some(id), None, || {
                "spacer() in a Wrap has no space to share; it is ignored".into()
            });
    }

    /// Pack the items, place them, and leave the scope closed. Does nothing for a scope
    /// that [`Self::begin_wrap`] did not begin.
    pub(crate) fn finish_wrap(&mut self) -> WrapLayout {
        self.finish_layout_item();
        if !self.wrapping() {
            return WrapLayout::default();
        }
        let flow = self.flow.take().expect("a wrapping scope has a flow");
        let rule = flow.wrap.expect("a wrapping scope has a rule");
        let id = self.scope;
        let mut bad = false;
        let measures: Vec<_> = flow
            .items
            .iter()
            .map(|item| {
                let mut measure = item.measure;
                let size = clean(measure.size);
                bad |= size != measure.size;
                measure.size = size;
                measure
            })
            .collect();
        if bad {
            self.context
                .report(crate::DiagnosticKind::InvalidValue, Some(id), None, || {
                    "a Wrap item has a non-finite or oversized extent; it is limited".into()
                });
        }
        let packed = pack(&measures, rule.width, rule.gap);
        let origin = self.layout.bounds.min;
        let mut items = Vec::with_capacity(measures.len());
        for ((item, measure), slot) in flow.items.into_iter().zip(&measures).zip(&packed.slots) {
            let line = packed.lines.get(slot.line).copied().unwrap_or_default();
            let offset = Vec2::new(
                slot.x + rule.lines.offset(rule.width - line.width),
                line.y + rule.items.offset(line.height - measure.size.y),
            );
            let target = origin + offset;
            if let Some(placement) = item.placement {
                self.context
                    .place(placement, target - item.origin, self.clip);
            }
            if !measure.gap {
                items.push(Rect::from_min_size(target, measure.size));
            }
        }
        let lines: Vec<_> = packed
            .lines
            .iter()
            .map(|line| {
                let x = rule.lines.offset(rule.width - line.width);
                Rect::from_min_size(
                    origin + Vec2::new(x, line.y),
                    Vec2::new(line.width, line.height),
                )
            })
            .collect();
        let widest = packed.lines.iter().map(|l| l.width).fold(0.0_f32, f32::max);
        let height = packed.lines.last().map_or(0.0, |l| l.y + l.height);
        let size = Vec2::new(rule.width.max(widest), height);
        self.layout.used = size;
        self.layout.cursor = origin + Vec2::new(0.0, height);
        WrapLayout { size, lines, items }
    }
}
