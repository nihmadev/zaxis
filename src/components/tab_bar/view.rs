//! Painting pass of the tabs: eased amounts for every tab first (the dividers need both
//! neighbours), then each tab's surface, content, tooltip and accessibility node.

use super::{
    access::{self, Described},
    drag::TabDnd,
    paint,
    row::Row,
};
use crate::{
    components::{drag_drop::style::DragStyle, Tooltip, Ui},
    Id, Rect, Response, Vec2,
};

pub(super) struct Frame<'a> {
    pub slots: &'a [Rect],
    pub ids: &'a [Id],
    pub close_ids: &'a [Id],
    pub responses: &'a [Response],
    pub closes: &'a [Option<Response>],
    pub enabled: &'a [bool],
    pub dnds: &'a [TabDnd],
    pub current: Option<usize>,
    pub focus: Option<usize>,
    pub landing: Option<usize>,
    /// A tab of the group is over the strip: its drop area shows.
    pub over: bool,
}

pub(super) fn paint_tabs(ui: &mut Ui<'_>, row: &mut Row<'_>, f: &Frame<'_>) {
    let look = row.look;
    let style = ui.style().clone();
    let n = row.entries.len();
    let hover_motion = style.motion.hover.clone();
    let drag_motion = DragStyle::default().tween(&style);
    let dim_to = DragStyle::default().resolve(&style).source_opacity;
    let list = access::list(ui, row.group.bar.with("list"), row.enabled);
    // Pass 1: eased amounts, which the dividers need from both neighbours.
    let mut selected = Vec::with_capacity(n);
    let mut hover = Vec::with_capacity(n);
    for i in 0..n {
        let key = row.entries[i].key;
        let r = f.responses[i];
        let on = f.enabled[i];
        let up = |b: bool| if b { 1.0_f32 } else { 0.0 };
        selected.push(
            ui.transition(("sel", key), up(f.current == Some(i)), hover_motion.clone())
                .value,
        );
        let hovered = on && (r.hovered || r.pressed) && ui.dragging().is_none();
        hover.push(
            ui.transition(("hov", key), up(hovered), hover_motion.clone())
                .value,
        );
    }
    let mut visuals = Vec::with_capacity(n);
    let mut gap_at = None;
    for i in 0..n {
        let key = row.entries[i].key;
        let shift_to = match f.landing {
            Some(at) if i >= at => look.insertion_gap,
            _ => 0.0,
        };
        let shift = ui
            .transition(("shift", key), shift_to, drag_motion.clone())
            .value;
        gap_at = gap_at.or((shift > 0.0).then_some(i));
        visuals.push(f.slots[i].translate(Vec2::new(shift, 0.0)));
    }
    let mut dividers = Vec::new();
    for i in 0..n {
        let strong = |j: usize| selected[j].max(hover[j]);
        let next = if i + 1 < n { strong(i + 1) } else { 0.0 };
        dividers.push(1.0 - strong(i).max(next));
    }
    if look.folder {
        paint::dividers(
            ui,
            row.group.bar.with("dividers"),
            &visuals,
            &dividers,
            look,
        );
    }
    // Pass 2: each tab.
    for i in 0..n {
        let key = row.entries[i].key;
        let r = f.responses[i];
        let on = f.enabled[i];
        let wants_close = row.entries[i].closable;
        let close_hit = f.closes[i];
        let target = if !on {
            look.text_disabled
        } else if f.current == Some(i) {
            look.text_selected
        } else if r.hovered || r.pressed {
            look.text_hover
        } else {
            look.text_idle
        };
        let text = ui
            .transition(("fg", key), target, hover_motion.clone())
            .value;
        let dim = ui
            .transition(
                ("dim", key),
                if f.dnds[i].dragging { dim_to } else { 1.0 },
                drag_motion.clone(),
            )
            .value;
        let focus_ring =
            (f.focus == Some(i) && r.focus_visible && on).then_some(style.focus_border);
        let view = paint::View {
            id: f.ids[i],
            slot: visuals[i],
            look,
            selected: selected[i],
            hover: hover[i],
            pressed: on && r.pressed,
            dim,
            focus: focus_ring,
        };
        paint::body(ui, &view);
        let close = wants_close.then(|| {
            let revealed = f.current == Some(i) || r.hovered || f.focus == Some(i);
            let shown = ui
                .transition(
                    ("close", key),
                    if revealed && on { 1.0_f32 } else { 0.0 },
                    hover_motion.clone(),
                )
                .value;
            let close_hover = close_hit.is_some_and(|c| c.hovered || c.pressed);
            let amount = ui
                .transition(
                    ("close-hover", key),
                    if close_hover { 1.0_f32 } else { 0.0 },
                    hover_motion.clone(),
                )
                .value;
            paint::Close {
                rect: paint::close_rect(visuals[i], look),
                shown,
                hover: amount,
                pressed: close_hit.is_some_and(|c| c.pressed),
                enabled: on,
            }
        });
        let weight = if f.current == Some(i) {
            look.weight_selected
        } else {
            look.weight
        };
        let icon = row.entries[i].icon.take();
        let label = row.plan.labels[i].as_ref();
        paint::content(ui, &view, icon, label, text, weight, close);
        let cut = label.is_none_or(|l| l.truncated);
        let tip = row.entries[i].tooltip.clone().or_else(|| {
            (cut && !row.entries[i].text.is_empty()).then(|| row.entries[i].text.clone())
        });
        if let Some(tip) = tip {
            Tooltip::new(tip).show(ui, r);
        }
        let name = row.entries[i].text.clone();
        let name = if name.is_empty() {
            row.entries[i].tooltip.clone().unwrap_or_default()
        } else {
            name
        };
        access::tab(
            ui,
            &Described {
                id: f.ids[i],
                rect: f.slots[i],
                name: &name,
                selected: f.current == Some(i),
                index: i,
                count: n,
                enabled: on,
                close: wants_close.then(|| (f.close_ids[i], paint::close_rect(f.slots[i], look))),
            },
        );
    }
    if let Some(at) = f.landing {
        let x = if at < n {
            f.slots[at].min.x
        } else {
            f.slots.last().map_or(row.area.min.x, |s| s.max.x)
        };
        let shown = ui
            .transition_from("insertion", 0.0_f32, 1.0_f32, drag_motion.clone())
            .value;
        let x = x + if gap_at.is_some() {
            look.insertion_gap * 0.5
        } else {
            0.0
        };
        paint::insertion(
            ui,
            row.group.bar.with("insertion"),
            x,
            row.strip,
            shown,
            look,
        );
    }
    let area = ui
        .transition(
            "drop-area",
            if f.over { 1.0_f32 } else { 0.0 },
            drag_motion.clone(),
        )
        .value;
    paint::drop_area(ui, row.group.bar.with("drop-area"), row.area, area, look);
    ui.a11y_end(list, Some(row.strip));
}
