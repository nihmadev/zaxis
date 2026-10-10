use super::{
    DockChild, DockError, DockEvent, DockFloat, DockIssue, DockNode, DockSide, DockState, PanelId,
};
use crate::{Id, Rect};

impl DockState {
    pub fn activate(&mut self, panel: PanelId) -> Result<DockEvent, DockError> {
        let Some(DockNode::Tabs { active, .. }) = self.group_mut(panel) else {
            return missing(panel);
        };
        *active = Some(panel);
        self.focused = Some(panel);
        Ok(DockEvent::Activated { panel })
    }
    pub fn close(&mut self, panel: PanelId) -> Result<DockEvent, DockError> {
        if !self.remove(panel) {
            return missing(panel);
        }
        self.normalize();
        Ok(DockEvent::Closed { panel })
    }
    /// Move into the group containing `target`, before `before` or at the end.
    pub fn move_panel(
        &mut self,
        panel: PanelId,
        target: PanelId,
        before: Option<PanelId>,
    ) -> Result<DockEvent, DockError> {
        let Some(group) = self.group_mut(target) else {
            return missing(target);
        };
        let group = group.id();
        if !self.remove(panel) {
            return missing(panel);
        }
        if let Some(DockNode::Tabs { panels, active, .. }) = self.group_id_mut(group) {
            let at = before
                .and_then(|b| panels.iter().position(|p| *p == b))
                .unwrap_or(panels.len());
            panels.insert(at, panel);
            *active = Some(panel);
        }
        self.focused = Some(panel);
        self.normalize();
        Ok(DockEvent::Moved { panel, target })
    }
    /// Detach a panel into an adjacent group. `ratio` is the new panel's fraction.
    pub fn split(
        &mut self,
        panel: PanelId,
        target: PanelId,
        side: DockSide,
        ratio: f32,
    ) -> Result<DockEvent, DockError> {
        if panel == target {
            return missing(target);
        }
        let Some(group) = self.group_mut(target) else {
            return missing(target);
        };
        let group_id = group.id();
        if !ratio.is_finite() {
            return Err(DockError(DockIssue::InvalidFraction(group_id)));
        }
        if !self.remove(panel) {
            return missing(panel);
        }
        let new_id = self.unique_node_id(("group", panel));
        let split_id = self.unique_node_id(("split", panel, group_id));
        if let Some(group) = self.group_id_mut(group_id) {
            let old = group.clone();
            let new = DockNode::Tabs {
                id: new_id,
                panels: vec![panel],
                active: Some(panel),
            };
            let ratio = ratio.clamp(0.05, 0.95);
            let mut children = vec![DockChild::new(old, 1.0 - ratio), DockChild::new(new, ratio)];
            if side.before() {
                children.reverse();
            }
            *group = DockNode::Split {
                id: split_id,
                axis: side.axis(),
                children,
            };
        }
        self.focused = Some(panel);
        self.normalize();
        Ok(DockEvent::Split {
            panel,
            target,
            side,
        })
    }
    pub fn float(&mut self, panel: PanelId, bounds: Rect) -> Result<DockEvent, DockError> {
        if !bounds.is_finite() || bounds.is_empty() {
            return Err(DockError(DockIssue::InvalidBounds(panel)));
        }
        if !self.remove(panel) {
            return missing(panel);
        }
        let id = self.unique_node_id(("float", panel));
        self.floats.push(DockFloat {
            node: DockNode::Tabs {
                id,
                panels: vec![panel],
                active: Some(panel),
            },
            bounds,
        });
        self.focused = Some(panel);
        self.normalize();
        Ok(DockEvent::Floated { panel })
    }
    pub fn dock_float(
        &mut self,
        panel: PanelId,
        target: PanelId,
        side: Option<DockSide>,
        ratio: f32,
    ) -> Result<DockEvent, DockError> {
        if !self.floats.iter().any(|f| f.node.contains(panel)) {
            return missing(panel);
        }
        match side {
            Some(side) => self.split(panel, target, side, ratio),
            None => self.move_panel(panel, target, None),
        }
    }
    /// Return a floating panel to an empty dock surface.
    pub fn dock_root(&mut self, panel: PanelId) -> Result<DockEvent, DockError> {
        if let Some(target) = self.root.as_ref().and_then(super::options::active) {
            return self.dock_float(panel, target, None, 0.5);
        }
        if !self.floats.iter().any(|f| f.node.contains(panel)) || !self.remove(panel) {
            return missing(panel);
        }
        let id = self.unique_node_id(("root", panel));
        self.root = Some(DockNode::Tabs {
            id,
            panels: vec![panel],
            active: Some(panel),
        });
        self.focused = Some(panel);
        self.normalize();
        Ok(DockEvent::LayoutChanged)
    }
    /// Reopen a panel. An empty Dock becomes a single group.
    pub fn open(
        &mut self,
        panel: PanelId,
        target: Option<PanelId>,
    ) -> Result<DockEvent, DockError> {
        if self.contains(panel) {
            return self.activate(panel);
        }
        if let Some(target) = target.or_else(|| self.panels().first().copied()) {
            let Some(DockNode::Tabs { panels, active, .. }) = self.group_mut(target) else {
                return missing(target);
            };
            panels.push(panel);
            *active = Some(panel);
        } else {
            self.root = Some(DockNode::tabs(("root", panel), [panel]));
        }
        self.focused = Some(panel);
        Ok(DockEvent::LayoutChanged)
    }
    fn remove(&mut self, panel: PanelId) -> bool {
        let Some(DockNode::Tabs { panels, active, .. }) = self.group_mut(panel) else {
            return false;
        };
        let Some(at) = panels.iter().position(|p| *p == panel) else {
            return false;
        };
        panels.remove(at);
        if *active == Some(panel) {
            *active = panels.get(at.min(panels.len().saturating_sub(1))).copied();
        }
        true
    }
    fn unique_node_id(&mut self, source: impl std::hash::Hash) -> Id {
        let mut id = Id::new(source);
        let mut salt = 0_u64;
        while self.group_id_mut(id).is_some() {
            id = id.with(salt);
            salt += 1;
        }
        id
    }
}
fn missing<T>(panel: PanelId) -> Result<T, DockError> {
    Err(DockError(DockIssue::MissingPanel(panel)))
}
