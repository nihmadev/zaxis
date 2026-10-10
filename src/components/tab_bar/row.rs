//! One pass over the tabs of a strip: keys, pointer and close events, drag and drop, then
//! painting and the accessibility nodes.

use super::{
    drag::{self, Group},
    input,
    layout::Plan,
    options::{TabActivation, TabMove, TabRelease},
    paint,
    style::Look,
    view,
};
use crate::{
    components::{Sense, Ui},
    DragReason, Id, ImageSource, Insertion, Rect, Response, Vec2,
};
use winit::keyboard::KeyCode;

pub(super) struct Entry {
    pub key: Id,
    pub tooltip: Option<String>,
    pub icon: Option<ImageSource>,
    pub enabled: bool,
    pub closable: bool,
    pub text: String,
}

pub(super) struct Row<'a> {
    pub look: &'a Look,
    pub plan: &'a Plan,
    pub entries: Vec<Entry>,
    pub selected: Option<usize>,
    pub activation: TabActivation,
    pub reorderable: bool,
    pub keyboard_close: bool,
    pub group: Group,
    pub enabled: bool,
    /// The strip, for centering the insertion line and the tail target.
    pub strip: Rect,
    /// Where tabs may land: the strip without what sits beside the row.
    pub area: Rect,
    pub scrolls: bool,
    /// The focus and selection the last pass ended with, to reveal what changed.
    pub last_selected: Option<Id>,
    pub last_focused: Option<Id>,
    pub window_drag: bool,
}

#[derive(Default)]
pub(super) struct Found {
    pub responses: Vec<Response>,
    pub picked: Option<usize>,
    pub closed: Option<usize>,
    pub moved: Option<TabMove>,
    pub released_outside: Option<TabRelease>,
    pub focused: Option<Id>,
    /// Space after the last tab that takes a window drag.
    pub tail: Option<Rect>,
}

/// Index of the first set flag.
fn first(flags: impl Iterator<Item = bool>) -> Option<usize> {
    flags.enumerate().find_map(|(i, f)| f.then_some(i))
}

pub(super) fn run(ui: &mut Ui<'_>, row: &mut Row<'_>, origin: Vec2) -> Found {
    let (look, n) = (row.look, row.entries.len());
    let mut found = Found::default();
    let mut x = origin.x;
    let slots: Vec<Rect> = row
        .plan
        .widths
        .iter()
        .map(|w| {
            let slot = Rect::from_min_size(Vec2::new(x, origin.y), Vec2::new(*w, look.height));
            x += w;
            slot
        })
        .collect();
    let keys: Vec<Id> = row.entries.iter().map(|e| e.key).collect();
    let ids: Vec<Id> = keys.iter().map(|k| ui.interact_id(("tab", *k))).collect();
    let closes: Vec<Id> = keys.iter().map(|k| ui.interact_id(("close", *k))).collect();
    let enabled: Vec<bool> = row
        .entries
        .iter()
        .map(|e| row.enabled && e.enabled)
        .collect();
    let can_close: Vec<bool> = row
        .entries
        .iter()
        .zip(&enabled)
        .map(|(e, on)| e.closable && *on)
        .collect();

    // Keys act before the hit regions are registered, so the tab they move the focus to
    // is the Tab stop in this very pass.
    let mut focus = ui
        .context
        .focused()
        .and_then(|f| ids.iter().position(|id| *id == f))
        .filter(|i| enabled[*i]);
    let mut picked = None;
    let mut closed = None;
    let pressed = ui.context.input().keys_pressed.clone();
    let modifiers = ui.context.input().modifiers;
    if let Some(from) = focus {
        if let Some(to) = input::target(&pressed, modifiers, from, &enabled) {
            focus = Some(to);
            ui.context.request_focus(ids[to]);
            if row.activation == TabActivation::OnFocus {
                picked = Some(to);
            }
        }
        if row.keyboard_close && pressed.contains(&KeyCode::Delete) && can_close[from] {
            closed = Some(from);
        }
        if let (true, Some(step)) = (row.reorderable, input::step(&pressed, modifiers)) {
            found.moved = step_move(row, &keys, from, step);
        }
    }
    let clicked = first(ids.iter().map(|id| ui.context.clicked(*id)));
    if let Some(i) = clicked.filter(|i| enabled[*i]) {
        picked = (row.selected != Some(i)).then_some(i);
        // A press with the mouse hands the strip's Tab stop to the tab, without a ring.
        ui.context.request_focus(ids[i]);
        focus = Some(i);
    }
    let current = picked.or(row.selected);
    let active = focus.or_else(|| {
        current
            .filter(|i| enabled[*i])
            .or_else(|| enabled.iter().position(|e| *e))
    });

    // Hit regions: the tab, then its close button above it.
    let mut responses = Vec::with_capacity(n);
    let mut close_hits: Vec<Option<Response>> = Vec::with_capacity(n);
    let mut dnds = Vec::with_capacity(n);
    for i in 0..n {
        let sense = if active == Some(i) {
            Sense::CLICK | Sense::FOCUS | Sense::MIDDLE
        } else {
            Sense::CLICK | Sense::MIDDLE
        };
        let allowed = enabled[i];
        let response =
            ui.add_enabled_ui(allowed, |ui| ui.interact(slots[i], ("tab", keys[i]), sense));
        let close = can_close[i].then(|| {
            let rect = paint::close_rect(slots[i], look);
            ui.interact(
                rect,
                ("close", keys[i]),
                Sense::CLICK | Sense::DRAG | Sense::MIDDLE,
            )
        });
        let dnd = drag::tab(
            ui,
            keys[i],
            response,
            &row.group,
            row.reorderable && allowed,
        );
        responses.push(response);
        close_hits.push(close);
        dnds.push(dnd);
    }
    if closed.is_none() {
        closed = (0..n).find(|i| {
            can_close[*i]
                && (close_hits[*i].is_some_and(|c| c.clicked() || c.middle_clicked())
                    || responses[*i].middle_clicked())
        });
    }
    if let Some(i) = closed.filter(|i| focus == Some(*i)) {
        if let Some(next) = input::tab_after_close(i, &enabled) {
            ui.context.request_focus(ids[next]);
        }
    }

    // Where a dragged tab would land, and what was dropped.
    let last = slots.last().copied();
    let tail_rect = last.map_or(
        Rect::from_min_max(
            origin,
            Vec2::new(row.area.max.x.max(origin.x), origin.y + look.height),
        ),
        |last| {
            Rect::from_min_max(
                Vec2::new(last.max.x, last.min.y),
                Vec2::new(row.area.max.x.max(last.max.x), last.max.y),
            )
        },
    );
    let mut hover_at = None;
    let mut dropped = None;
    for (i, dnd) in dnds.iter_mut().enumerate() {
        if let Some(zone) = dnd.hover {
            hover_at = Some(if zone == Insertion::Before { i } else { i + 1 });
        }
        if let Some(d) = dnd.dropped.take() {
            let at = if d.insertion == Some(Insertion::Before) {
                i
            } else {
                i + 1
            };
            dropped = Some((d.payload, at));
        }
    }
    if !row.scrolls && tail_rect.size().x > 0.0 {
        let (hovering, tail_drop) = drag::tail(ui, tail_rect, &row.group);
        if hovering {
            hover_at = Some(n);
        }
        if let Some(d) = tail_drop {
            dropped = Some((d.payload, n));
        }
        if row.window_drag && ui.is_enabled() {
            let id = ui.scope.with("native-drag");
            ui.context.register_hit(crate::context::HitRegion {
                id,
                window: ui.window,
                rect: tail_rect,
                clip: ui.clip,
                action: crate::context::HitAction::NativeDrag,
            });
        }
        found.tail = Some(tail_rect);
    }
    if let Some((payload, at)) = dropped {
        let before = keys.get(at).copied();
        let moved = TabMove {
            tab: payload.tab,
            from_bar: payload.bar,
            to_bar: row.group.bar,
            before,
        };
        let same = payload.bar == row.group.bar;
        if !same || moved.reorder(&keys).is_some() {
            found.moved = Some(moved);
        }
    }
    let dragged = dnds.iter().position(|d| d.dragging);
    // A drop beside the dragged tab changes nothing: no gap, no line.
    let landing = hover_at.filter(|at| dragged.is_none_or(|from| *at != from && *at != from + 1));
    if let Some((i, finished)) = dnds
        .iter()
        .enumerate()
        .find_map(|(i, d)| d.finished.map(|f| (i, f)))
    {
        if finished.reason == DragReason::Cancelled {
            let seen = ui
                .context
                .containers
                .tab_bars
                .values()
                .any(|s| s.window == ui.window && s.strip.contains(finished.position));
            if !seen {
                found.released_outside = Some(TabRelease {
                    tab: keys[i],
                    position: finished.position,
                });
            }
        }
    }

    // Reveal what changed: a new selection, a focus that moved, a tab the user picked.
    if row.scrolls {
        let selected_now = current.map(|i| keys[i]);
        let focused_now = focus.map(|i| keys[i]);
        let reveal = picked
            .or_else(|| {
                (selected_now != row.last_selected)
                    .then_some(current)
                    .flatten()
            })
            .or_else(|| (focused_now != row.last_focused).then_some(focus).flatten());
        if let Some(i) = reveal {
            ui.scroll_to_response(&responses[i]);
        }
    }

    view::paint_tabs(
        ui,
        row,
        &view::Frame {
            slots: &slots,
            ids: &ids,
            close_ids: &closes,
            responses: &responses,
            closes: &close_hits,
            enabled: &enabled,
            dnds: &dnds,
            current,
            focus,
            landing,
            over: hover_at.is_some(),
        },
    );
    found.responses = responses;
    for (i, response) in found.responses.iter_mut().enumerate() {
        response.changed = picked == Some(i);
    }
    found.picked = picked;
    found.closed = closed;
    found.focused = focus.map(|i| keys[i]);
    found
}

/// Ctrl+Shift with an arrow key: the focused tab moves one place.
fn step_move(row: &Row<'_>, keys: &[Id], from: usize, step: i32) -> Option<TabMove> {
    let to = from
        .checked_add_signed(step as isize)
        .filter(|to| *to < keys.len())?;
    // In front of the tab that follows its new place.
    let before = if step < 0 {
        Some(keys[to])
    } else {
        keys.get(to + 1).copied()
    };
    Some(TabMove {
        tab: keys[from],
        from_bar: row.group.bar,
        to_bar: row.group.bar,
        before,
    })
}
