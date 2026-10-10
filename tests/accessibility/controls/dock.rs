use crate::support::*;

struct Viewer;
impl DockViewer for Viewer {
    fn title(&self, p: &PanelId) -> String {
        if *p == Id::new("editor") {
            "Editor".into()
        } else {
            "Files".into()
        }
    }
    fn ui(&mut self, ui: &mut Ui<'_>, p: &PanelId) {
        ui.button(format!("Inside {}", self.title(p)));
    }
}
fn build(c: &mut Context, state: &mut DockState) -> DockOutput {
    Root::new().show(c, |ui| {
        Dock::new("dock", state).name("Workspace").show(ui, Viewer)
    })
}
#[test]
fn dock_describes_root_tablist_active_panel_and_close_actions() {
    let mut h = Harness::new();
    let mut style = h.context.style().clone();
    style.motion.reduced_motion = true;
    h.context.set_style(style);
    let mut state = DockState::new(DockNode::tabs(
        "group",
        [Id::new("editor"), Id::new("files")],
    ));
    h.pass(|c| build(c, &mut state));
    h.tree.expect(Role::Group, "Workspace");
    assert_eq!(h.tree.all(Role::TabList).len(), 1);
    let editor = h.tree.expect(Role::Tab, "Editor");
    let files = h.tree.expect(Role::Tab, "Files");
    assert_eq!(h.tree.node(editor).is_selected(), Some(true));
    assert!(h
        .tree
        .all(Role::Group)
        .iter()
        .any(|id| h.tree.name(*id) == "Editor"));
    assert!(h.act(files, Action::Click));
    let mut events = Vec::new();
    h.pass(|c| {
        events.extend(build(c, &mut state).events);
    });
    h.pass(|c| build(c, &mut state));
    assert_eq!(state.focused, Some(Id::new("files")));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, DockEvent::Activated { .. }))
            .count(),
        1
    );
    let close = h.tree.expect(Role::Button, "Close Files");
    assert!(h.act(close, Action::Click));
    h.settle(|c| build(c, &mut state));
    assert!(!state.contains(Id::new("files")));
    assert_eq!(h.tree.all(Role::Tab).len(), 1);
    assert_eq!(h.tree.all(Role::TabList).len(), 1);
}
