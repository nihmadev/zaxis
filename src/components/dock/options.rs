use super::{DockEvent::*, DockNode, DockSide, DockState, DockStyle, PanelId};
use crate::{Id, ImageSource, Rect, Ui, Vec2};
use std::hash::Hash;

pub trait DockViewer {
    fn title(&self, panel: &PanelId) -> String;
    fn icon(&self, _panel: &PanelId) -> Option<ImageSource> {
        None
    }
    fn closable(&self, _panel: &PanelId) -> bool {
        true
    }
    fn can_float(&self, _panel: &PanelId) -> bool {
        true
    }
    fn ui(&mut self, ui: &mut Ui<'_>, panel: &PanelId);
}
pub(super) struct ClosureViewer<F>(pub F);
impl<F: FnMut(&mut Ui<'_>, &PanelId)> DockViewer for ClosureViewer<F> {
    fn title(&self, panel: &PanelId) -> String {
        format!("Panel {:x}", panel.value())
    }
    fn ui(&mut self, ui: &mut Ui<'_>, panel: &PanelId) {
        (self.0)(ui, panel);
    }
}
/// Action IDs from the application's existing registry/keymap. No default close/split keys.
#[derive(Clone, Copy, Debug, Default)]
pub struct DockActions {
    pub close: Option<Id>,
    pub split_right: Option<Id>,
    pub split_down: Option<Id>,
}

pub struct Dock<'a> {
    pub(super) id: Id,
    pub(super) state: &'a mut DockState,
    pub(super) style: DockStyle,
    pub(super) size: Option<Vec2>,
    pub(super) name: String,
    pub(super) actions: DockActions,
}
impl<'a> Dock<'a> {
    pub fn new(source: impl Hash, state: &'a mut DockState) -> Self {
        Self {
            id: Id::new(source),
            state,
            style: DockStyle::default(),
            size: None,
            name: "Dock".into(),
            actions: DockActions::default(),
        }
    }
    pub fn style(mut self, style: DockStyle) -> Self {
        self.style = style;
        self
    }
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size.is_finite().then_some(size.max(Vec2::ZERO));
        self
    }
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }
    pub fn actions(mut self, actions: DockActions) -> Self {
        self.actions = actions;
        self
    }
    pub fn show(self, ui: &mut Ui<'_>, mut viewer: impl DockViewer) -> DockOutput {
        ui.layout_item(|ui| super::show::run(self, ui, &mut viewer))
    }
}
impl<V: DockViewer + ?Sized> DockViewer for &mut V {
    fn title(&self, p: &PanelId) -> String {
        (**self).title(p)
    }
    fn icon(&self, p: &PanelId) -> Option<ImageSource> {
        (**self).icon(p)
    }
    fn closable(&self, p: &PanelId) -> bool {
        (**self).closable(p)
    }
    fn can_float(&self, p: &PanelId) -> bool {
        (**self).can_float(p)
    }
    fn ui(&mut self, ui: &mut Ui<'_>, p: &PanelId) {
        (**self).ui(ui, p);
    }
}
impl Ui<'_> {
    pub fn dock(
        &mut self,
        state: &mut DockState,
        viewer: impl FnMut(&mut Ui<'_>, &PanelId),
    ) -> DockOutput {
        let id = self.auto_id("dock");
        Dock::new(id, state).show(self, ClosureViewer(viewer))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum DockEvent {
    Closed {
        panel: PanelId,
    },
    Moved {
        panel: PanelId,
        target: PanelId,
    },
    Split {
        panel: PanelId,
        target: PanelId,
        side: DockSide,
    },
    Floated {
        panel: PanelId,
    },
    Activated {
        panel: PanelId,
    },
    LayoutChanged,
}
impl DockEvent {
    pub fn panel(&self) -> Option<PanelId> {
        match self {
            Closed { panel }
            | Moved { panel, .. }
            | Split { panel, .. }
            | Floated { panel }
            | Activated { panel } => Some(*panel),
            LayoutChanged => None,
        }
    }
}
#[derive(Clone, Debug)]
pub struct DockPanelOutput {
    pub panel: PanelId,
    pub group: Id,
    pub bounds: Rect,
    pub target_bounds: Rect,
    pub content_bounds: Rect,
    pub floating: bool,
}
#[derive(Clone, Debug, Default)]
pub struct DockOutput {
    pub events: Vec<DockEvent>,
    pub panels: Vec<DockPanelOutput>,
    pub bounds: Rect,
    pub focus_ring: Option<Rect>,
    pub preview: Option<Rect>,
    pub focus_ring_color: Option<crate::Color>,
}
pub(super) fn active(node: &DockNode) -> Option<PanelId> {
    match node {
        DockNode::Tabs {
            active: selected,
            panels,
            ..
        } => selected.or_else(|| panels.first().copied()),
        DockNode::Split { children, .. } => children.iter().find_map(|c| active(&c.node)),
    }
}
