//! The always-visible part of a menu: title buttons, or a single hamburger button.
use super::{Kind, MenuItem};
use crate::{
    components::{Button, ButtonVariant, Ui},
    AccessAction, AccessActionKind, AccessNode, AccessRole, Color, Id, Padding, PaintMode, Rect,
    Shape, Vec2,
};

/// What the triggers did in one pass.
pub(super) struct Triggers {
    /// Each trigger's rectangle; separators have none.
    pub anchors: Vec<Option<Rect>>,
    /// Index of the trigger activated in this pass.
    pub pressed: Option<usize>,
    /// Assistive technology asked to close the open menu.
    pub collapse: bool,
}

/// A trigger that opens a panel says so, and whether it is open. While it is, a click goes
/// where the pointer's would: to the popup's proxy of the trigger, which closes the menu.
fn opener(node: &mut AccessNode, bar: Id, open: bool) {
    node.toggled = None;
    node.has_popup()
        .expanded(open)
        .action(AccessActionKind::Expand)
        .action(AccessActionKind::Collapse);
    if open {
        node.clicks(bar).controls(bar.with(("level", 0)));
    }
}

/// Requests to open (first) or close (second) the panel of trigger `id`.
fn requests(ui: &mut Ui<'_>, id: Id, open: bool) -> (bool, bool) {
    let mut asked = (false, false);
    for request in ui.context.take_access_actions(id) {
        match request {
            AccessAction::Expand if !open => asked.0 = true,
            AccessAction::Collapse if open => asked.1 = true,
            _ => {}
        }
    }
    asked
}

/// Draws the triggers of the menu `bar`; `open_index` is the one whose panel is shown.
pub(super) fn show(
    ui: &mut Ui<'_>,
    bar: Id,
    items: &[MenuItem],
    compact: bool,
    open_index: Option<usize>,
) -> Triggers {
    if compact {
        let side = ui.style().control_height;
        let open = open_index.is_some();
        let node = ui.context.a11y_len();
        let response = ui.add(
            Button::new("##menu")
                .id_source(bar)
                .variant(ButtonVariant::Ghost)
                .padding(Padding::all(0.0))
                .min_size(Vec2::splat(side))
                .selected(open)
                .painter(PaintMode::After, |painter, info| {
                    let color = info.style.foreground.unwrap_or(Color::WHITE);
                    let center = info.bounds.center();
                    for dy in [-4.0, 0.0, 4.0] {
                        painter.paint(Shape::Line {
                            start: center + Vec2::new(-6.0, dy),
                            end: center + Vec2::new(6.0, dy),
                            width: 1.5,
                            color,
                        });
                    }
                }),
        );
        if let Some(node) = ui.context.a11y_node_mut(node) {
            node.label("Menu");
            opener(node, bar, open);
        }
        let (expand, collapse) = requests(ui, response.id, open);
        return Triggers {
            anchors: vec![Some(response.rect)],
            pressed: (response.clicked() || expand).then_some(0),
            collapse,
        };
    }
    let mut triggers = Triggers {
        anchors: vec![None; items.len()],
        pressed: None,
        collapse: false,
    };
    let scope = ui.a11y_begin(bar.with("bar"), AccessRole::MenuBar, |_| {});
    ui.horizontal(|ui| {
        for (index, item) in items.iter().enumerate() {
            if item.is_separator() {
                continue;
            }
            let open = open_index == Some(index);
            let node = ui.context.a11y_len();
            let response = ui.add(
                Button::new(item.text.as_str())
                    .id_source((bar, index))
                    .variant(ButtonVariant::Ghost)
                    .enabled(item.enabled)
                    .selected(open),
            );
            triggers.anchors[index] = Some(response.rect);
            let submenu = matches!(item.kind, Kind::Submenu(_));
            if let Some(node) = ui.context.a11y_node_mut(node) {
                node.role(AccessRole::MenuItem).toggled = None;
                if submenu {
                    opener(node, bar, open);
                }
            }
            let (expand, collapse) = requests(ui, response.id, open);
            if response.clicked() || expand && submenu {
                triggers.pressed = Some(index);
            }
            triggers.collapse |= collapse;
        }
    });
    ui.a11y_end(scope, None);
    triggers
}
