use super::{
    layout::Group, options::DockActions, DockError, DockEvent, DockNode, DockOutput, DockSide,
    DockState, DockViewer, PanelId,
};
use crate::{ContextMenu, ContextMenuItem, Id, Rect, Response, Ui};
use winit::keyboard::KeyCode;

pub(super) enum Request {
    Activate(PanelId),
    Close(PanelId),
    Others(PanelId),
    Move(PanelId, PanelId, Option<PanelId>),
    Split(PanelId, PanelId, DockSide),
    Float(PanelId, Rect),
    Root(PanelId),
}
pub(super) fn apply(
    ui: &mut Ui<'_>,
    state: &mut DockState,
    requests: Vec<Request>,
    viewer: &dyn DockViewer,
    output: &mut DockOutput,
    runtime: &mut super::DockRuntime,
) {
    for request in requests {
        let event = match request {
            Request::Activate(p) => {
                let selected = matches!(state.group_mut(p),Some(DockNode::Tabs { active,.. }) if *active == Some(p));
                if state.focused == Some(p) && selected {
                    continue;
                }
                state.activate(p)
            }
            Request::Close(p) if viewer.closable(&p) => state.close(p),
            Request::Root(p) => state.dock_root(p),
            Request::Move(p, t, b) => state.move_panel(p, t, b),
            Request::Split(p, t, s) => state.split(p, t, s, 0.5),
            Request::Float(p, r) if viewer.can_float(&p) => state.float(p, r),
            Request::Others(p) => {
                let panels = match state.group_mut(p) {
                    Some(DockNode::Tabs { panels, .. }) => panels.clone(),
                    _ => continue,
                };
                let mut changed = false;
                for other in panels
                    .into_iter()
                    .filter(|other| *other != p && viewer.closable(other))
                {
                    changed |= state.close(other).is_ok();
                }
                if changed {
                    Ok(DockEvent::LayoutChanged)
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        match event {
            Ok(event) => {
                if matches!(
                    event,
                    DockEvent::Moved { .. }
                        | DockEvent::Split { .. }
                        | DockEvent::Floated { .. }
                        | DockEvent::Closed { .. }
                        | DockEvent::LayoutChanged
                ) {
                    runtime.pending_focus = state.focused;
                }
                output.events.push(event);
                ui.context.request_repaint();
            }
            Err(DockError(issue)) => {
                ui.context
                    .report(crate::DiagnosticKind::InvalidValue, None, None, || {
                        format!("Dock: {issue:?}")
                    })
            }
        }
    }
}
pub(super) fn menu(
    ui: &mut Ui<'_>,
    p: PanelId,
    response: Response,
    others: Option<PanelId>,
    viewer: &dyn DockViewer,
    requests: &mut Vec<Request>,
) {
    let items = [
        ContextMenuItem::new("close", "Close").enabled(viewer.closable(&p)),
        ContextMenuItem::new("others", "Close others").enabled(others.is_some()),
        ContextMenuItem::new("float", "Float").enabled(viewer.can_float(&p)),
        ContextMenuItem::new("right", "Split right").enabled(others.is_some()),
        ContextMenuItem::new("down", "Split down").enabled(others.is_some()),
    ];
    if let Some(action) = ContextMenu::new(("dock-tab", p), &items)
        .show(ui, response)
        .selected
    {
        let request = if action == Id::new("close") {
            Some(Request::Close(p))
        } else if action == Id::new("others") {
            Some(Request::Others(p))
        } else if action == Id::new("float") {
            Some(Request::Float(
                p,
                Rect::from_min_size(response.rect.min, crate::Vec2::new(380.0, 260.0)),
            ))
        } else {
            others.map(|t| {
                Request::Split(
                    p,
                    t,
                    if action == Id::new("right") {
                        DockSide::Right
                    } else {
                        DockSide::Bottom
                    },
                )
            })
        };
        requests.extend(request);
    }
}
pub(super) fn keyboard(
    ui: &mut Ui<'_>,
    state: &DockState,
    groups: &[Group],
    actions: DockActions,
    runtime: &super::DockRuntime,
    requests: &mut Vec<Request>,
) {
    let focused = ui.context.focused();
    let owns = focused.is_some_and(|id| runtime.focus.values().any(|ids| ids.contains(&id)));
    if !owns || ui.context.dragging().is_some() || !ui.enabled {
        return;
    }
    let Some(panel) = state.focused else { return };
    let current = groups.iter().find(|g| g.panels.contains(&panel));
    let Some(current) = current else { return };
    let others = current.panels.iter().copied().find(|p| *p != panel);
    if actions.close.is_some_and(|a| ui.actions().triggered(a)) {
        requests.push(Request::Close(panel));
    }
    for (action, side) in [
        (actions.split_right, DockSide::Right),
        (actions.split_down, DockSide::Bottom),
    ] {
        if action.is_some_and(|a| ui.actions().triggered(a)) {
            if let Some(t) = others {
                requests.push(Request::Split(panel, t, side));
            }
        }
    }
    let input = ui.context.input();
    let nav = input.modifiers.control_key() && input.modifiers.alt_key();
    let moving = input.modifiers.control_key() && input.modifiers.shift_key();
    if !nav && !moving {
        return;
    }
    for (key, side) in [
        (KeyCode::ArrowLeft, DockSide::Left),
        (KeyCode::ArrowRight, DockSide::Right),
        (KeyCode::ArrowUp, DockSide::Top),
        (KeyCode::ArrowDown, DockSide::Bottom),
    ] {
        if !ui.context.input().keys_pressed.contains(&key) {
            continue;
        }
        if let Some(next) = nearest(groups, current.id, side) {
            if moving {
                requests.push(Request::Move(panel, next.active, None));
            } else {
                requests.push(Request::Activate(next.active));
                if let Some(id) = runtime.tab_focus.get(&next.active) {
                    ui.context.request_focus(*id);
                }
            }
        }
    }
}
fn nearest(groups: &[Group], current: Id, side: DockSide) -> Option<&Group> {
    let from = groups.iter().find(|g| g.id == current)?.bounds.center();
    let score = |g: &Group| {
        let delta = g.bounds.center() - from;
        let main = match side {
            DockSide::Left => -delta.x,
            DockSide::Right => delta.x,
            DockSide::Top => -delta.y,
            DockSide::Bottom => delta.y,
        };
        let cross = side.axis().cross(delta).abs();
        (main > 0.1).then_some(main + cross * 2.0)
    };
    groups
        .iter()
        .filter(|g| g.id != current && score(g).is_some())
        .min_by(|a, b| {
            score(a)
                .unwrap_or(f32::INFINITY)
                .total_cmp(&score(b).unwrap_or(f32::INFINITY))
        })
}

impl crate::Context {
    /// Reserve Dock navigation before a child text field/tree/slider takes the arrows.
    pub(crate) fn dock_navigation_key(&self, code: KeyCode) -> bool {
        if !matches!(
            code,
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown
        ) || !self.input().modifiers.control_key()
            || !(self.input().modifiers.alt_key() || self.input().modifiers.shift_key())
            || self.popups.is_active()
        {
            return false;
        }
        self.focused().is_some_and(|id| {
            self.containers
                .docks
                .values()
                .any(|dock| dock.focus.values().any(|hits| hits.contains(&id)))
        })
    }
}
