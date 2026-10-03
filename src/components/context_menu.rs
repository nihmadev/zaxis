//! Pointer-anchored menus built on the shared popup layer.
use std::hash::Hash;

use super::{Popup, Response, ScrollArea, Ui};
use crate::winit::keyboard::KeyCode;
use crate::{
    context::{HitAction, HitRegion},
    Id, Padding, Rect, Vec2,
};

mod cache;
mod paint;

/// A stable action ID is independent of its visible label. Separators have no ID.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuItem {
    pub id: Option<Id>,
    pub text: String,
    pub icon: String,
    pub icon_shapes: Vec<crate::Shape>,
    pub left_text: String,
    pub right_text: String,
    pub enabled: bool,
}

impl ContextMenuItem {
    pub fn new(id: impl Hash, text: impl Into<String>) -> Self {
        Self {
            id: Some(Id::new(id)),
            text: text.into(),
            icon: String::new(),
            icon_shapes: Vec::new(),
            left_text: String::new(),
            right_text: String::new(),
            enabled: true,
        }
    }
    pub fn separator() -> Self {
        Self {
            id: None,
            text: String::new(),
            icon: String::new(),
            icon_shapes: Vec::new(),
            left_text: String::new(),
            right_text: String::new(),
            enabled: false,
        }
    }
    /// A font glyph or short symbol, in a dedicated aligned icon column.
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = icon.into();
        self.icon_shapes.clear();
        self
    }
    /// Vector icon drawn in an 18 × 18 logical-pixel box, with local coordinates.
    pub fn icon_shapes(mut self, shapes: impl IntoIterator<Item = crate::Shape>) -> Self {
        self.icon_shapes = shapes.into_iter().collect();
        self.icon.clear();
        self
    }
    pub fn left_text(mut self, text: impl Into<String>) -> Self {
        self.left_text = text.into();
        self
    }
    pub fn right_text(mut self, text: impl Into<String>) -> Self {
        self.right_text = text.into();
        self
    }
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Colors inherit the current UI theme; these overrides control menu proportions
/// and reuse the existing popup and control appearance styles.
#[derive(Clone, Debug)]
pub struct ContextMenuStyle {
    pub min_width: f32,
    pub row_height: f32,
    pub font_size: f32,
    pub separator_height: f32,
    pub padding: Padding,
    pub popup: crate::PopupStyle,
    pub item: crate::ControlStyle,
}
impl Default for ContextMenuStyle {
    fn default() -> Self {
        Self {
            min_width: 220.0,
            row_height: 24.0,
            font_size: 14.0,
            separator_height: 9.0,
            padding: Padding::all(4.0),
            popup: Default::default(),
            item: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ContextMenuOutput {
    pub open: bool,
    pub rect: Option<Rect>,
    /// Exactly one activation per gesture. The menu closes before returning it.
    pub selected: Option<Id>,
}

#[derive(Default)]
pub(crate) struct MenuState {
    open: bool,
    position: Vec2,
    active: Option<Id>,
    pointer: Option<Vec2>,
    keyboard: bool,
    cache: Option<cache::MenuCache>,
    pub(crate) last_frame: u64,
}

pub struct ContextMenu<'a> {
    source: Id,
    items: &'a [ContextMenuItem],
    style: ContextMenuStyle,
    position: Option<Vec2>,
}
impl<'a> ContextMenu<'a> {
    pub fn new(source: impl Hash, items: &'a [ContextMenuItem]) -> Self {
        Self {
            source: Id::new(source),
            items,
            style: Default::default(),
            position: None,
        }
    }
    pub fn style(mut self, style: ContextMenuStyle) -> Self {
        self.style = style;
        self
    }
    /// Open programmatically on this pass. Omit on later passes to retain state.
    pub fn open_at(mut self, position: Vec2) -> Self {
        assert!(position.is_finite());
        self.position = Some(position);
        self
    }
    /// Attach after building the target, in the same UI scope. Works with passive
    /// text as well as controls, and follows deferred layout and scroll clipping.
    /// Action IDs must be unique within this menu.
    pub fn show(self, ui: &mut Ui<'_>, target: Response) -> ContextMenuOutput {
        let style = &self.style;
        assert!([
            style.min_width,
            style.row_height,
            style.separator_height,
            style.font_size
        ]
        .iter()
        .all(|v| v.is_finite() && *v > 0.0));
        assert!([
            style.padding.left,
            style.padding.right,
            style.padding.top,
            style.padding.bottom
        ]
        .iter()
        .all(|v| v.is_finite() && *v >= 0.0));
        let id = ui.scope.with(("context-menu", self.source, target.id));
        let anchor_id = id.with("anchor");
        let hit = HitRegion {
            id: anchor_id,
            window: ui.window,
            rect: target.rect,
            clip: ui.clip,
            action: HitAction::ContextMenu,
        };
        if !ui.attach_tooltip_anchor(target.id, hit) {
            ui.context.register_hit(hit);
        }
        let popup_id = Popup::id(ui, Id::new(id));
        let mut state = ui.context.context_menus.remove(&id).unwrap_or_default();
        state.last_frame = ui.context.frame;
        if ui.context.dismissed_popups.remove(&popup_id) {
            state.open = false;
        }
        let pointer_open = ui
            .context
            .secondary_target
            .filter(|(anchor, _)| *anchor == anchor_id)
            .map(|(_, p)| p);
        if let Some(position) = self.position.or(pointer_open) {
            state.position = position;
            state.keyboard = false;
            state.open = ui.enabled;
            state.active = self
                .items
                .iter()
                .find(|i| i.enabled && i.id.is_some())
                .and_then(|i| i.id);
            if pointer_open.is_some() {
                ui.context.secondary_target = None;
            }
            ui.context.request_repaint();
        }
        state.open &= ui.enabled && !self.items.is_empty();
        let mut output = ContextMenuOutput::default();
        if state.open {
            let pointer = ui.context.input().pointer;
            let pointer_moved = pointer != state.pointer;
            state.pointer = pointer;
            if pointer_moved {
                state.keyboard = false;
            }
            let cache = match state.cache.take() {
                Some(cache) if cache.matches(self.items, style) => cache,
                _ => cache::MenuCache::new(ui, self.items, style),
            };
            let enabled = &cache.enabled;
            if !state.active.is_some_and(|id| enabled.contains(&id)) {
                state.active = enabled.first().copied();
            }
            let mut reveal = false;
            for key in ui.context.combo_input.remove(&id).unwrap_or_default() {
                let index = state
                    .active
                    .and_then(|a| enabled.iter().position(|i| *i == a))
                    .unwrap_or(0);
                let next = match key {
                    KeyCode::ArrowDown => Some((index + 1) % enabled.len().max(1)),
                    KeyCode::ArrowUp => {
                        Some((index + enabled.len().saturating_sub(1)) % enabled.len().max(1))
                    }
                    KeyCode::Home => Some(0),
                    KeyCode::End => Some(enabled.len().saturating_sub(1)),
                    KeyCode::Enter | KeyCode::Space => {
                        output.selected = state.active;
                        None
                    }
                    _ => None,
                };
                if let Some(next) = next {
                    state.active = enabled.get(next).copied();
                    state.keyboard = true;
                    reveal = true;
                }
            }
            let metrics = &cache.metrics;
            let mut popup_style = style.popup;
            popup_style.spacing = Some(0.0);
            popup_style
                .surface
                .rounding
                .get_or_insert(crate::CornerRadius::all(5.0));
            popup_style.surface.shadow.get_or_insert(crate::Shadow {
                color: crate::Color::rgba(0, 0, 0, 48),
                offset: Vec2::new(0.0, 3.0),
                blur_radius: 8.0,
                spread: 0.0,
            });
            let foreground = ui.style().text_color.0;
            popup_style.surface.border.get_or_insert(crate::Border::new(
                1.0,
                crate::Color::rgba(foreground[0], foreground[1], foreground[2], 22),
            ));
            let mut popup = Popup::new(id, Rect::from_min_size(state.position, Vec2::ZERO))
                .size(Vec2::new(
                    metrics.width + style.padding.size().x,
                    metrics.height + style.padding.size().y,
                ))
                .gap(0.0)
                .padding(style.padding)
                .style(popup_style);
            // The shared keyboard queue preserves multiple key events per redraw.
            popup.key_target = Some(id);
            let active = state.keyboard.then_some(state.active).flatten();
            let shown = popup.show(ui, &mut state.open, |ui| {
                let mut scroll = ui.style().scroll;
                scroll.padding = Padding::all(0.0);
                scroll.spacing = 0.0;
                scroll.bar_margin = 0.0;
                let mut scroll_area = ScrollArea::vertical()
                    .id_source(id.with("scroll"))
                    .max_height(ui.available_height())
                    .overlay_scrollbars(true)
                    .style(scroll)
                    .show_hints(false)
                    .middle_mouse_scroll(false);
                if reveal {
                    if let Some(index) = self
                        .items
                        .iter()
                        .position(|item| item.id == active && item.id.is_some())
                    {
                        scroll_area = scroll_area.scroll_to_rect(Rect::from_min_size(
                            Vec2::new(0.0, cache.offsets[index]),
                            Vec2::new(metrics.width, style.row_height),
                        ));
                    }
                }
                scroll_area.show(ui, |ui| {
                    ui.layout.spacing = 0.0;
                    let origin = ui.layout.bounds.min;
                    let visible = ui.clip_rect();
                    let first = cache
                        .offsets
                        .partition_point(|y| *y <= visible.min.y - origin.y)
                        .saturating_sub(1)
                        .min(self.items.len());
                    let end = cache
                        .offsets
                        .partition_point(|y| *y < visible.max.y - origin.y)
                        .min(self.items.len())
                        .max(first);
                    ui.add_space(cache.offsets[first]);
                    for item in &self.items[first..end] {
                        let (response, selected) = paint::row(ui, id, item, active, metrics, style);
                        if selected {
                            output.selected = item.id;
                        }
                        if let Some(response) = response {
                            if pointer_moved && response.hovered && response.enabled {
                                state.active = item.id;
                            }
                        }
                    }
                    ui.add_space(metrics.height - cache.offsets[end]);
                });
            });
            output.rect = shown.map(|p| p.rect);
            if output.selected.is_some() {
                ui.context.close_popup();
                state.open = false;
            }
            state.cache = Some(cache);
        }
        output.open = state.open;
        ui.context.context_menus.insert(id, state);
        output
    }
}

impl Ui<'_> {
    pub fn context_menu(
        &mut self,
        target: Response,
        items: &[ContextMenuItem],
    ) -> ContextMenuOutput {
        ContextMenu::new(target.id, items).show(self, target)
    }
}
