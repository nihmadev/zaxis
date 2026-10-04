//! Cascading menu panels: rows, extra panel surfaces and placement.
use super::MenuItem;
use crate::{
    components::{
        appearance::Appearance,
        context_menu::{paint, ContextMenuItem, ContextMenuStyle},
        ScrollArea, Ui,
    },
    context::{HitAction, HitRegion},
    layout::LayoutCursor,
    Align, CornerRadius, Id, Layout, Padding, Rect, Vec2,
};

/// What happened in one panel during this pass.
#[derive(Default)]
pub(super) struct Events {
    /// The pointer moved onto this enabled row.
    pub hovered: Option<usize>,
    pub clicked: Option<usize>,
    /// Screen rectangles of the rows that open a submenu.
    pub submenu_rows: Vec<(usize, Rect)>,
}

pub(super) fn rows(items: &[MenuItem]) -> Vec<ContextMenuItem> {
    items
        .iter()
        .enumerate()
        .map(|(index, item)| item.row(index))
        .collect()
}

/// Top edge of every row inside the panel content, plus the total height.
fn offsets(rows: &[ContextMenuItem], style: &ContextMenuStyle) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(rows.len() + 1);
    offsets.push(0.0);
    for row in rows {
        let height = if row.id.is_some() {
            style.row_height
        } else {
            style.separator_height
        };
        offsets.push(offsets.last().unwrap() + height);
    }
    offsets
}

/// Rows of one panel, scrollable when the panel is shorter than its content.
#[allow(clippy::too_many_arguments)]
pub(super) fn body(
    ui: &mut Ui<'_>,
    panel: Id,
    rows: &[ContextMenuItem],
    metrics: &paint::Metrics,
    style: &ContextMenuStyle,
    active: Option<Id>,
    reveal: Option<usize>,
    pointer_moved: bool,
) -> Events {
    let mut events = Events::default();
    let mut scroll = ui.style().scroll;
    scroll.padding = Padding::all(0.0);
    scroll.spacing = 0.0;
    scroll.bar_margin = 0.0;
    let mut area = ScrollArea::vertical()
        .id_source(panel.with("scroll"))
        .max_height(ui.available_height())
        .overlay_scrollbars(true)
        .style(scroll)
        .show_hints(false)
        .middle_mouse_scroll(false);
    if let Some(index) = reveal {
        let top = offsets(rows, style)[index];
        area = area.scroll_to_rect(Rect::from_min_size(
            Vec2::new(0.0, top),
            Vec2::new(metrics.width, style.row_height),
        ));
    }
    area.show(ui, |ui| {
        ui.layout.spacing = 0.0;
        for (index, item) in rows.iter().enumerate() {
            let (response, clicked) = paint::row(ui, panel, item, active, metrics, style);
            let Some(response) = response else { continue };
            if pointer_moved && response.hovered && response.enabled {
                events.hovered = Some(index);
            }
            if clicked {
                events.clicked = Some(index);
            }
            if item.submenu {
                events.submenu_rows.push((index, response.rect));
            }
        }
    });
    events
}

/// Where a submenu opens: beside its parent row, flipped to the left or shifted up when
/// the viewport has no room.
pub(super) fn submenu_rect(
    parent: Rect,
    row: Rect,
    size: Vec2,
    padding: Padding,
    viewport: Rect,
) -> Rect {
    let size = Vec2::new(
        size.x.min(viewport.size().x),
        size.y.min((viewport.size().y - 8.0).max(0.0)),
    );
    let right = parent.max.x - 2.0;
    let x = if right + size.x <= viewport.max.x {
        right
    } else {
        (parent.min.x + 2.0 - size.x).max(viewport.min.x)
    };
    let y = (row.min.y - padding.top)
        .min(viewport.max.y - size.y - 4.0)
        .max(viewport.min.y + 4.0);
    Rect::from_min_size(Vec2::new(x, y), size)
}

/// An extra popup-layer panel next to the root popup: same surface as `Popup`, own layer
/// and hit block. The root popup stays the only owner of focus, keys and dismissal.
pub(super) fn panel<R>(
    ui: &mut Ui<'_>,
    layer: Id,
    rect: Rect,
    surface: crate::PopupStyle,
    padding: Padding,
    build: impl FnOnce(&mut Ui<'_>) -> R,
) -> R {
    let style = ui.style().clone();
    let mut component = style.popup;
    component.merge(surface);
    let viewport = ui.context.viewport();
    let mut body = Appearance::new(style.window_fill, style.border, style.text_color);
    body.rounding = CornerRadius::all(4.0);
    body.blur = style.blur_radius;
    body.shadow = style.elevation;
    body.opacity = style.opacity;
    body.apply(component.surface);
    ui.context.push_popup_layer(layer);
    ui.context.register_hit(HitRegion {
        id: layer.with("block"),
        window: layer,
        rect,
        clip: viewport,
        action: HitAction::Block,
    });
    let mut paint = Vec::new();
    body.paint_shadow(rect, body.rounding, &mut paint);
    body.paint_body(rect, body.rounding, &style, body.blur, &mut paint);
    ui.context.paint_blur(
        layer.with("blur"),
        layer,
        rect.intersect(viewport),
        crate::Blur::new(rect)
            .radius(body.blur)
            .corner_radius(body.rounding),
    );
    ui.context
        .paint(layer.with("body"), layer, rect.intersect(viewport), paint);
    let mut child_style = style.clone();
    child_style.text_color = body.text_color;
    let mut child = Ui {
        flow: None,
        context: ui.context,
        window: layer,
        scope: layer,
        sequence: 0,
        clip: padding.inset(rect).intersect(viewport),
        layout: LayoutCursor::new(padding.inset(rect), Layout::Vertical, 0.0),
        enabled: ui.enabled,
        backdrop_blur: body.blur,
        hover_style: ui.hover_style,
        local_style: Some(std::sync::Arc::new(child_style)),
        local_style_revision: ui.local_style_revision,
    };
    child.begin_layout(Align::Start);
    let inner = build(&mut child);
    child.finish_layout();
    inner
}
