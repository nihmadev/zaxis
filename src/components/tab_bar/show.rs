//! One pass of a strip: measure, lay out, then hand the row to [`row`] inside either a
//! clipped region or a horizontal scroll area.

use std::{collections::HashSet, hash::Hash};

use super::{
    drag::Group,
    layout::{self, Item},
    menu::{self, Choice},
    options::TabBarOutput,
    paint, row,
    row::{Entry, Found, Row},
    state::TabBarState,
    style::{snap, Look},
    TabBar,
};
use crate::{
    components::{visible_label, ScrollArea, Ui},
    context::invalid_value,
    Padding, Rect, Vec2,
};

pub(super) fn run<T: PartialEq + Hash>(bar: TabBar<'_, T>, ui: &mut Ui<'_>) -> TabBarOutput {
    if ui.flow.is_some() {
        return ui.layout_item(|ui| run(bar, ui));
    }
    let id = bar.cfg.id;
    ui.push_id(("tab-bar", id), |ui| pass(bar, ui))
}

fn pass<T: PartialEq + Hash>(bar: TabBar<'_, T>, ui: &mut Ui<'_>) -> TabBarOutput {
    let TabBar {
        selected,
        mut tabs,
        cfg,
        trailing,
    } = bar;
    let style = ui.style().clone();
    let scale = match ui.context.scale_factor() {
        s if s.is_finite() && s > 0.0 => s,
        _ => 1.0,
    };
    let look = Look::resolve(&style, &cfg, scale);
    let gid = ui.scope;
    let enabled = ui.is_enabled();

    let mut seen = HashSet::new();
    let before = tabs.len();
    tabs.retain(|t| seen.insert(t.key()));
    if tabs.len() != before {
        invalid_value(
            "TabBar::new",
            format!(
                "{} tabs repeat an earlier value; ignored",
                before - tabs.len()
            ),
        );
    }
    let selected_at = tabs.iter().position(|t| t.value == *selected);
    let entries: Vec<Entry> = tabs
        .iter_mut()
        .map(|t| {
            let visible = visible_label(&t.label).to_owned();
            Entry {
                key: t.key(),
                tooltip: t.tooltip.take(),
                icon: t.icon.take(),
                enabled: t.enabled,
                closable: t.closable.unwrap_or(cfg.closable),
                text: visible,
            }
        })
        .collect();
    let items: Vec<Item> = entries
        .iter()
        .map(|e| Item {
            text: e.text.clone(),
            has_icon: e.icon.is_some(),
            closable: e.closable,
        })
        .collect();

    // Whatever sits beside the row is measured first: it keeps its width.
    let available = ui.available_width();
    let mut trailing_size = Vec2::ZERO;
    let mut trailing_place = None;
    if let Some(build) = trailing {
        let (_, size, placement) = ui.measure_effect_as(gid.with("trailing"), enabled, build);
        trailing_size = size;
        trailing_place = Some(placement);
    }
    let reserved = if trailing_size.x > 0.0 {
        trailing_size.x + style.spacing.min(8.0)
    } else {
        0.0
    };
    let mut plan = layout::plan(ui.context, &items, &look, cfg.width, available - reserved);
    let menu_width = look.close_size + 2.0 * look.plate_inset.x + 6.0;
    let menu = cfg.overflow_menu && plan.overflow;
    if menu {
        plan = layout::plan(
            ui.context,
            &items,
            &look,
            cfg.width,
            available - reserved - menu_width,
        );
    }
    let natural = plan.total + reserved + if menu { menu_width } else { 0.0 };
    let width = if available.is_finite() {
        available
    } else {
        natural
    };
    let at = ui.layout.cursor;
    let strip = ui.allocate_space(Vec2::new(width, look.height));
    let strip = Rect::from_min_size(
        Vec2::new(snap(strip.min.x, scale), snap(strip.min.y, scale)),
        strip.size(),
    );
    paint::strip(ui, gid.with("strip"), strip, &look);
    if let Some(placement) = trailing_place {
        let to = Vec2::new(
            strip.max.x - trailing_size.x,
            strip.min.y + (look.height - look.line - trailing_size.y) * 0.5,
        );
        let clip = ui.clip_rect();
        ui.context.place(placement, to - at, clip);
    }
    let area = Rect::from_min_max(
        strip.min,
        Vec2::new(
            (strip.max.x - reserved - if menu { menu_width } else { 0.0 }).max(strip.min.x),
            strip.max.y,
        ),
    );

    let state = ui.context.containers.tab_bars.remove(&gid);
    let mut menu_open = state.as_ref().is_some_and(|s| s.menu_open);
    let mut row = Row {
        look: &look,
        plan: &plan,
        entries,
        selected: selected_at,
        activation: cfg.activation,
        reorderable: cfg.reorderable,
        keyboard_close: cfg.keyboard_close,
        group: Group {
            group: cfg.group.unwrap_or(cfg.id),
            bar: cfg.id,
        },
        enabled,
        strip,
        area,
        scrolls: plan.overflow,
        last_selected: state.as_ref().and_then(|s| s.selected),
        last_focused: state.as_ref().and_then(|s| s.focused),
        window_drag: cfg.window_drag,
    };
    let found: Found = if plan.overflow {
        let mut scroll = style.scroll;
        scroll.padding = Padding::all(0.0);
        scroll.spacing = 0.0;
        ScrollArea::horizontal()
            .id_source("tabs")
            .style(scroll)
            .content_width(plan.total)
            .overlay_scrollbars(true)
            .show_at(ui, area, None, |ui| {
                let content = ui.allocate_space(Vec2::new(plan.total, look.height));
                row::run(ui, &mut row, content.min)
            })
            .inner
    } else {
        let clip = ui.clip;
        ui.clip = clip.intersect(area);
        let found = row::run(ui, &mut row, area.min);
        ui.clip = clip;
        found
    };

    let mut found = found;
    let mut picked = found.picked;
    if menu {
        let anchor = Rect::from_min_max(
            Vec2::new(area.max.x, strip.min.y),
            Vec2::new(area.max.x + menu_width, strip.max.y - look.line),
        );
        let choices: Vec<Choice> = row
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| Choice {
                label: e.text.clone(),
                selected: Some(i) == picked.or(selected_at),
                enabled: enabled && e.enabled,
            })
            .collect();
        if let Some(i) = menu::show(ui, anchor, &mut menu_open, &choices, &look) {
            picked = Some(i).filter(|i| selected_at != Some(*i));
            found.picked = picked;
        }
    } else {
        menu_open = false;
    }

    let mut selected_key = selected_at.map(|i| row.entries[i].key);
    let mut changed = false;
    if let Some(i) = picked {
        if let Some(tab) = tabs.into_iter().nth(i) {
            *selected = tab.value;
            selected_key = Some(row.entries[i].key);
            changed = true;
        }
    }
    let visible = strip.intersect(ui.clip_rect());
    ui.context.containers.tab_bars.insert(
        gid,
        TabBarState {
            last_frame: ui.context.frame,
            window: ui.window,
            strip: visible,
            selected: selected_key,
            focused: found.focused,
            menu_open,
        },
    );
    let mut whole = ui.response(gid.with("strip"), strip, false);
    whole.changed = changed;
    let selected_id = picked.map(|i| row.entries[i].key);
    let closed = found.closed.map(|i| row.entries[i].key);
    TabBarOutput {
        responses: found.responses,
        strip: whole,
        free_space: found.tail,
        selected: selected_id,
        closed,
        moved: found.moved,
        released_outside: found.released_outside,
    }
}
