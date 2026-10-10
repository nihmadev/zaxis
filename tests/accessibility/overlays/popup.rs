use super::*;

#[derive(Default)]
struct Panel {
    open: bool,
    applied: u32,
}

impl Panel {
    fn build(&mut self, context: &mut Context) {
        Window::new("Main").show(context, |ui| {
            let options = ui.button("Options");
            if options.clicked() {
                self.open = !self.open;
            }
            Popup::new("options", options.rect).show(ui, &mut self.open, |ui| {
                ui.label("Density");
                if ui.button("Apply").clicked() {
                    self.applied += 1;
                }
            });
        });
    }
}

#[test]
fn a_plain_popup_is_an_unnamed_layer_around_its_content() {
    let mut harness = still();
    let mut panel = Panel::default();
    harness.pass(|ctx| panel.build(ctx));
    assert!(harness.tree.find(Role::Button, "Apply").is_none());
    let options = harness.tree.expect(Role::Button, "Options");
    harness.act(options, Action::Click);
    harness.settle(|ctx| panel.build(ctx));
    let tree = &harness.tree;
    let apply = tree.expect(Role::Button, "Apply");
    let layer = tree.parent(apply).expect("the popup's layer");
    assert_eq!(
        tree.node(layer).role(),
        Role::GenericContainer,
        "structure only"
    );
    assert_eq!(tree.parent(layer), Some(tree.root()));
    assert_eq!(
        tree.node(tree.root()).children().last(),
        Some(&layer),
        "above the window"
    );
    assert!(inside(tree, tree.expect(Role::Label, "Density"), layer));
    assert!(!inside(tree, apply, tree.expect(Role::Window, "Main")));
    assert_eq!(harness.pass(|ctx| panel.build(ctx)), None);
    assert!(harness.context.diagnostics().is_empty());

    assert!(harness.act(apply, Action::Click));
    harness.settle(|ctx| panel.build(ctx));
    assert_eq!(panel.applied, 1);
    assert!(panel.open, "a click inside does not close it");
    key(&mut harness.context, KeyCode::Escape);
    harness.pass(|ctx| panel.build(ctx));
    assert!(
        harness.tree.find(Role::Button, "Apply").is_none(),
        "gone with the pass that closed it"
    );
    assert!(harness.tree.find(Role::Label, "Density").is_none());
    assert_eq!(harness.tree.node(harness.tree.root()).children().len(), 1);
    harness.settle(|ctx| panel.build(ctx));
    assert!(!panel.open);
    assert_eq!(harness.pass(|ctx| panel.build(ctx)), None);
}

#[derive(Default)]
struct Editor {
    open: bool,
    more: bool,
    applied: u32,
}

impl Editor {
    fn build(&mut self, context: &mut Context) {
        Window::new("Main").show(context, |ui| {
            let edit = ui.button("Edit");
            if edit.clicked() {
                self.open = !self.open;
            }
            Popup::new("editor", edit.rect).show(ui, &mut self.open, |ui| {
                let more = ui.button("More");
                if more.clicked() {
                    self.more = !self.more;
                }
                Popup::new("more", more.rect).show(ui, &mut self.more, |ui| {
                    if ui.button("Apply").clicked() {
                        self.applied += 1;
                    }
                });
            });
        });
    }
}

#[test]
fn nested_popups_are_layers_above_their_parents_and_keep_them() {
    let mut harness = still();
    let mut editor = Editor::default();
    harness.pass(|ctx| editor.build(ctx));
    let edit = harness.tree.expect(Role::Button, "Edit");
    harness.act(edit, Action::Click);
    harness.settle(|ctx| editor.build(ctx));
    let more = harness.tree.expect(Role::Button, "More");
    assert!(harness.act(more, Action::Click));
    harness.settle(|ctx| editor.build(ctx));
    let tree = &harness.tree;
    let apply = tree.expect(Role::Button, "Apply");
    let child_layer = tree.parent(apply).expect("the child's layer");
    let parent_layer = tree.parent(tree.expect(Role::Button, "More")).unwrap();
    assert_ne!(child_layer, parent_layer);
    let layers = tree.node(tree.root()).children();
    let rank = |layer: NodeId| layers.iter().position(|l| *l == layer);
    assert!(rank(child_layer) > rank(parent_layer), "the child is above");
    assert!(harness.context.diagnostics().is_empty());

    assert!(harness.act(apply, Action::Click));
    harness.settle(|ctx| editor.build(ctx));
    assert_eq!((editor.applied, editor.open, editor.more), (1, true, true));
    key(&mut harness.context, KeyCode::Escape);
    harness.settle(|ctx| editor.build(ctx));
    assert!(harness.tree.find(Role::Button, "Apply").is_none());
    assert!(harness.tree.find(Role::Button, "More").is_some(), "the parent stays");
    assert!(!editor.more && editor.open);
    assert_eq!(harness.pass(|ctx| editor.build(ctx)), None);
}
