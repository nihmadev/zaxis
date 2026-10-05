//! Keyboard drag: pick up a focused source, move between insertion points,
//! confirm or cancel. Nothing is intercepted before the pick-up, so text fields,
//! scroll areas and nested controls keep Space and the arrow keys.
use super::super::{Context, HitAction};
use super::state::{Hover, Session};
use crate::components::drag_drop::DragReason;
use crate::Vec2;
use winit::{event::ElementState, keyboard::KeyCode, window::CursorIcon};

fn navigation(code: KeyCode) -> bool {
    matches!(
        code,
        KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Space
            | KeyCode::Enter
            | KeyCode::NumpadEnter
    )
}

impl Context {
    /// Returns `true` when the key belongs to drag and drop.
    pub(crate) fn drag_key(&mut self, code: KeyCode, state: ElementState, repeat: bool) -> bool {
        let pressed = state == ElementState::Pressed;
        let Some(session) = &self.drag.session else {
            return pressed && !repeat && code == KeyCode::Space && self.drag_pick_up();
        };
        if code == KeyCode::Escape {
            if pressed {
                self.drag_cancel(DragReason::Escape);
            }
            return true;
        }
        if !session.keyboard {
            return false;
        }
        if !pressed {
            return navigation(code);
        }
        match code {
            KeyCode::ArrowDown | KeyCode::ArrowRight => self.drag_step(1),
            KeyCode::ArrowUp | KeyCode::ArrowLeft => self.drag_step(-1),
            KeyCode::Home => self.drag_jump(false),
            KeyCode::End => self.drag_jump(true),
            KeyCode::Space | KeyCode::Enter | KeyCode::NumpadEnter if !repeat => {
                self.drag_confirm()
            }
            KeyCode::Tab => {
                // Focus traversal continues; the drag does not outlive its focus.
                self.drag_cancel(DragReason::Cancelled);
                return false;
            }
            _ => return false,
        }
        true
    }

    /// Plain Space picks up a source that owns a focus stop. A source attached to
    /// another focusable control (a button, a tree) needs Ctrl+Space, because the
    /// control keeps plain Space for itself. Text input never starts a drag.
    fn drag_pick_up(&mut self) -> bool {
        let Some(focus) = self.interaction.focused else {
            return false;
        };
        let owner = self
            .interaction
            .previous_hits
            .iter()
            .find(|hit| hit.id == focus)
            .map(|hit| hit.action);
        if matches!(
            owner,
            None | Some(
                HitAction::TextEdit
                    | HitAction::ComboBox
                    | HitAction::DragValue
                    | HitAction::Slider
                    | HitAction::SplitResize { .. }
            )
        ) {
            return false;
        }
        let ctrl = self.input.modifiers.control_key();
        let Some((hit, info)) = self.interaction.previous_hits.iter().find_map(|hit| {
            let HitAction::DragSource { slot } = hit.action else {
                return None;
            };
            let info = *self.drag.last_sources.get(slot as usize)?;
            (info.focus == Some(focus) && info.own_focus != ctrl).then_some((*hit, info))
        }) else {
            return false;
        };
        let visible = hit.rect.intersect(hit.clip);
        if visible.is_empty() {
            return false;
        }
        self.drag.pending = None;
        self.drag.session = Some(Session {
            info,
            window: hit.window,
            rect: hit.rect,
            grab: Vec2::ZERO,
            pointer: visible.center(),
            keyboard: true,
            outside: false,
            payload: None,
            started: false,
            seen: false,
            hover: None,
            cursor: CursorIcon::Grabbing,
            snapshot: None,
            last_tick: None,
        });
        self.request_repaint();
        true
    }

    /// Reachable insertion points in reading order: only targets that accept the
    /// payload, are visible, are not covered, and win at their own point.
    fn drag_stops(&self) -> Vec<(Vec2, Hover)> {
        let mut stops = Vec::new();
        for hit in &self.interaction.previous_hits {
            let HitAction::DropTarget { slot } = hit.action else {
                continue;
            };
            let Some(info) = self
                .drag
                .last_targets
                .get(slot as usize)
                .filter(|i| i.accepts)
            else {
                continue;
            };
            let visible = hit.rect.intersect(hit.clip);
            if visible.is_empty() {
                continue;
            }
            let ts: Vec<f32> = match info.zones {
                None => vec![0.5],
                Some(zones) if zones.inside && zones.edge < 0.5 => {
                    let edge = (zones.edge / 2.0).max(0.02);
                    vec![edge, 0.5, 1.0 - edge]
                }
                Some(_) => vec![0.25, 0.75],
            };
            let vertical = info
                .zones
                .is_none_or(|zones| zones.layout == crate::Layout::Vertical);
            for t in ts {
                let mut point = hit.rect.center();
                if info.zones.is_some() {
                    if vertical {
                        point.y = hit.rect.min.y + hit.rect.size().y * t;
                    } else {
                        point.x = hit.rect.min.x + hit.rect.size().x * t;
                    }
                }
                point = point.clamp(
                    visible.min,
                    (visible.max - Vec2::splat(0.01)).max(visible.min),
                );
                if let Some(hover) = self.drag_resolve(point).filter(|h| h.id == info.id) {
                    if info.zones.is_none() || hover.insertion.is_some() {
                        stops.push((point, hover));
                    }
                }
            }
        }
        stops.sort_by(|a, b| a.0.y.total_cmp(&b.0.y).then(a.0.x.total_cmp(&b.0.x)));
        stops.dedup_by(|a, b| a.1.id == b.1.id && a.1.insertion == b.1.insertion);
        stops
    }

    fn drag_jump(&mut self, last: bool) {
        if !self.drag.session.as_ref().is_some_and(|s| s.started) {
            return;
        }
        let stops = self.drag_stops();
        if let Some((point, _)) = if last { stops.last() } else { stops.first() } {
            self.drag_move_virtual(*point);
        }
    }

    fn drag_step(&mut self, direction: i32) {
        let Some(session) = self.drag.session.as_ref().filter(|s| s.started) else {
            return;
        };
        let (pointer, hover) = (session.pointer, session.hover);
        let stops = self.drag_stops();
        let current = hover.and_then(|h| {
            stops
                .iter()
                .position(|(_, s)| s.id == h.id && s.insertion == h.insertion)
        });
        let next = match current {
            Some(i) => i
                .checked_add_signed(direction as isize)
                .filter(|n| *n < stops.len()),
            None if direction > 0 => stops
                .iter()
                .position(|(p, _)| p.y >= pointer.y)
                .or(if stops.is_empty() { None } else { Some(0) }),
            None => stops
                .iter()
                .rposition(|(p, _)| p.y < pointer.y)
                .or(stops.len().checked_sub(1)),
        };
        match next {
            Some(n) => self.drag_move_virtual(stops[n].0),
            None => self.drag_scroll_edge(direction),
        }
    }

    /// Past the last visible insertion point: scroll the container under the
    /// virtual pointer so the next press can reach newly built rows.
    fn drag_scroll_edge(&mut self, direction: i32) {
        let Some(pointer) = self.drag.session.as_ref().map(|s| s.pointer) else {
            return;
        };
        let area = self
            .scrolling
            .previous_order
            .iter()
            .rev()
            .copied()
            .find(|id| {
                let s = &self.scrolling.states[id];
                s.enabled && s.clip.contains(pointer)
            });
        if let Some(area) = area {
            let step = self.style.font_size.max(8.0) * 2.0 * direction as f32;
            self.scroll_from(area, Vec2::new(0.0, step), false);
        }
    }

    fn drag_move_virtual(&mut self, point: Vec2) {
        if let Some(session) = self.drag.session.as_mut() {
            session.pointer = point;
            session.outside = false;
        }
        self.drag_refresh_hover();
        self.request_repaint();
    }
}
