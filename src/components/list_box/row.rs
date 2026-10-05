//! One pass over the rows near the viewport: surfaces, content placement, pointer input.
use super::{
    model::{ListEntry, ListEntryKind, ListModel},
    nav::mods,
    paint::{self, Look, RowPaint},
    select::Selector,
    ListEvent, ListMode,
};
use crate::{
    components::{disclosure, scroll_area::RowMetrics, Loader, Sense, Text, Ui},
    Align, Color, DragSource, DropTarget, DropZones, Id, Insertion, Layout, Rect, RowDrag, Vec2,
};

/// A row handed to your content closure. The closure runs inside the row's padding, in a
/// horizontal layout centered vertically; use [`Self::color`] and [`Self::muted`] so text
/// follows the selected and disabled states.
pub struct ListRow<'a> {
    pub index: usize,
    pub key: Id,
    pub text: &'a str,
    pub selected: bool,
    /// The keyboard-active row of a focused list.
    pub active: bool,
    pub enabled: bool,
    pub color: Color,
    pub muted: Color,
    /// The area available to the content, in layout coordinates.
    pub bounds: Rect,
}
impl ListRow<'_> {
    /// Build content flush with the right edge of the row (a badge, a button). It is placed
    /// after being measured and keeps normal input priority over the row.
    pub fn trailing<R>(&self, ui: &mut Ui<'_>, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        ui.context.begin_placement(ui.window);
        ui.context.visual_depth += 1;
        let scope = ui.scope.with("trailing");
        let (result, used) = {
            let mut child = disclosure::child(ui, scope, self.bounds, Layout::Horizontal);
            child.begin_layout(Align::Center);
            let result = build(&mut child);
            child.finish_layout();
            (result, child.layout.used)
        };
        ui.context.visual_depth -= 1;
        let placement = ui.context.end_placement();
        let delta = Vec2::new((self.bounds.size().x - used.x).max(0.0), 0.0);
        ui.context
            .place(placement, delta, ui.clip.intersect(self.bounds));
        result
    }
}

pub(super) type Content<'c> = &'c mut dyn FnMut(&mut Ui<'_>, ListRow<'_>);

pub(super) struct Pass<'p, M: ListModel> {
    pub sel: Selector<'p, M>,
    pub id: Id,
    pub owner: Id,
    pub look: &'p Look,
    pub enabled: bool,
    pub focused: bool,
    pub focus_visible: bool,
    pub measured: bool,
    pub drag: bool,
    pub loading: bool,
    pub empty: &'p str,
    pub built: usize,
    /// Entries that are not items, for an item's position among the items.
    pub plain: Vec<usize>,
    /// The accessibility node of the keyboard-active row, when that row was built.
    pub active_node: Option<Id>,
}

impl<M: ListModel> Pass<'_, M> {
    /// Build entry `index` in `bounds`; returns the used height for measured rows.
    pub fn row(
        &mut self,
        ui: &mut Ui<'_>,
        index: usize,
        bounds: Rect,
        content: Content<'_>,
    ) -> Option<f32> {
        let model = self.sel.model;
        self.built += 1;
        if index >= model.len() {
            footer(ui, bounds);
            return None;
        }
        let entry = model.entry(index);
        match entry.kind {
            ListEntryKind::Header => {
                paint::header(ui, bounds, entry.text, self.look);
                let id = ui.scope.with("header");
                ui.a11y(id, bounds, crate::AccessRole::Label, |node| {
                    node.value(entry.text);
                });
            }
            ListEntryKind::Separator => paint::separator(ui, bounds, self.look),
            ListEntryKind::Item => return self.item(ui, index, entry, bounds, content),
        }
        None
    }

    fn item(
        &mut self,
        ui: &mut Ui<'_>,
        index: usize,
        entry: ListEntry<'_>,
        bounds: Rect,
        content: Content<'_>,
    ) -> Option<f32> {
        let look = self.look;
        let key = entry.key;
        let on = self.enabled && entry.enabled && ui.enabled;
        let selected = self.sel.set.contains(&key);
        let active = self.sel.state.active == Some(key);
        let checks = self.sel.mode == ListMode::Checks;
        let pad = look.margin;
        let left = bounds.min.x + pad + look.pad.left + if checks { look.check + 8.0 } else { 0.0 };
        let area = Rect::from_min_max(
            Vec2::new(left, bounds.min.y + pad + look.pad.top),
            Vec2::new(
                (bounds.max.x - pad - look.pad.right).max(left),
                (bounds.max.y - pad - look.pad.bottom).max(bounds.min.y + pad + look.pad.top),
            ),
        );
        let color = match (on, selected) {
            (false, _) => look.disabled,
            (true, true) => look.selected_text,
            (true, false) => look.text,
        };
        let muted = if on { color.with_opacity(0.7) } else { color };
        // Registered before the content's own hits exist, so nested controls win. A measured
        // row's height is exact from the pass after its first measurement.
        let hit = ui.add_enabled_ui(on, |ui| ui.interact(bounds, "hit", Sense::CLICK));
        // A click request is the click of the row's own region: same selection, same event.
        let access = ui.a11y_begin(hit.id, crate::AccessRole::ListBoxOption, |node| {
            let before = self.plain.partition_point(|plain| *plain < index);
            let items = self.sel.model.len().saturating_sub(self.plain.len());
            node.label(entry.text)
                .selected(selected)
                .position_in_set(index - before, items)
                .disabled(!on)
                .clicks(hit.id);
        });
        if active {
            self.active_node = Some(hit.id);
        }
        ui.context.begin_placement(ui.window);
        ui.context.visual_depth += 1;
        let scope = ui.scope.with("content");
        let used = {
            let mut cell = disclosure::child(ui, scope, area, Layout::Horizontal);
            cell.enabled &= on;
            cell.begin_layout(Align::Center);
            content(
                &mut cell,
                ListRow {
                    index,
                    key,
                    text: entry.text,
                    selected,
                    active: active && self.focused,
                    enabled: on,
                    color,
                    muted,
                    bounds: area,
                },
            );
            cell.finish_layout();
            cell.layout.used
        };
        ui.context.visual_depth -= 1;
        let placement = ui.context.end_placement();

        let frame = 2.0 * (pad) + look.pad.top + look.pad.bottom;
        let height = if self.measured {
            let h = (used.y + frame).max(look.min_row);
            (h * look.scale).ceil() / look.scale
        } else {
            bounds.size().y
        };
        let rect = Rect::from_min_size(bounds.min, Vec2::new(bounds.size().x, height));
        paint::row_surface(
            ui,
            rect,
            look,
            RowPaint {
                selected,
                hovered: on && hit.hovered,
                striped: index % 2 == 1,
                list_focused: self.focused,
            },
        );
        if checks {
            let at = bounds.min.x + pad + look.pad.left;
            paint::check_box(ui, rect, at, selected, color, look);
        }
        if on {
            self.pointer(ui, index, key, hit);
        }
        let dy = if self.measured {
            ((height - frame - used.y) * 0.5).max(0.0)
        } else {
            0.0
        };
        ui.context
            .place(placement, Vec2::new(0.0, dy), ui.clip.intersect(rect));
        ui.a11y_end(access, Some(rect));
        if active && self.focus_visible {
            paint::ring(ui, rect, look);
        }
        if self.drag && on {
            self.attach_drag(ui, key, active, hit);
        }
        self.measured.then_some(height)
    }

    fn pointer(&mut self, ui: &mut Ui<'_>, index: usize, key: Id, hit: crate::Response) {
        if (hit.pressed || hit.clicked()) && ui.context.focused_widget != Some(self.owner) {
            ui.context.request_focus(self.owner);
        }
        if hit.secondary_clicked() {
            self.sel.state.set_active(index, self.sel.model);
            self.sel.state.reveal = false;
            self.sel.events.push(ListEvent::Context(key));
        } else if hit.double_clicked() {
            self.sel.events.push(ListEvent::Activated(key));
        } else if hit.clicked() {
            self.sel.click(index, mods(ui.context.input().modifiers));
        }
    }

    fn attach_drag(&mut self, ui: &mut Ui<'_>, key: Id, active: bool, hit: crate::Response) {
        let list = self.id;
        DragSource::new(
            key,
            RowDrag {
                owner: list,
                row: key,
            },
        )
        .keyboard(active && self.focused)
        .focus_owner(self.owner)
        .attach(ui, hit);
        let drop = DropTarget::new(key, move |p: &RowDrag| p.owner == list)
            .zones(DropZones::rows())
            .attach(ui, hit);
        if let Some(drop) = drop.dropped {
            self.sel.events.push(ListEvent::Moved {
                key: drop.payload.row,
                target: key,
                position: drop.insertion.unwrap_or(Insertion::After),
            });
        }
    }

    /// Drawn above the rows: the pinned section header, or the empty state.
    pub fn overlay(&mut self, ui: &mut Ui<'_>, visible: Rect, rows: &dyn RowMetrics) {
        let look = self.look;
        let model = self.sel.model;
        if model.is_empty() {
            if !self.loading {
                let font = (look.font, look.weight, look.muted);
                paint::text(ui, self.empty, visible, font, true);
                if !self.empty.is_empty() {
                    let id = ui.scope.with("empty");
                    ui.a11y(id, visible, crate::AccessRole::Label, |node| {
                        node.value(self.empty);
                    });
                }
            }
            return;
        }
        let origin = ui.layout.bounds.min;
        let view_top = (visible.min.y - origin.y).max(0.0);
        let sections = rows.sections();
        let after = sections.partition_point(|h| *h <= rows.row_at(view_top));
        let Some(&header) = after.checked_sub(1).and_then(|i| sections.get(i)) else {
            return;
        };
        if rows.top(header) >= view_top {
            return;
        }
        let mut y = view_top;
        if let Some(next) = sections.get(after) {
            y = y.min(rows.top(*next) - look.header);
        }
        let row = Rect::from_min_size(
            origin + Vec2::new(0.0, y),
            Vec2::new(ui.layout.bounds.size().x, look.header),
        );
        let id = ui.next_id("sticky");
        paint::sticky(ui, row, model.entry(header).text, look, id);
    }
}

fn footer(ui: &mut Ui<'_>, bounds: Rect) {
    let side = (bounds.size().y * 0.6).clamp(8.0, 24.0);
    let scope = ui.scope.with("loader");
    let mut cell = disclosure::child(ui, scope, bounds, Layout::Horizontal);
    cell.begin_layout(Align::Center);
    cell.add_space((bounds.size().x - side) * 0.5);
    cell.add(Loader::new().size(side));
    cell.finish_layout();
}

/// Plain text content for rows without a custom builder.
pub(super) fn text_row(ui: &mut Ui<'_>, row: ListRow<'_>) {
    ui.add(Text::new(row.text).color(row.color).wrap(false));
}
