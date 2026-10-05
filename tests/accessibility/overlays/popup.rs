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
