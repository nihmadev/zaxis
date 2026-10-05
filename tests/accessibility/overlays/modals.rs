use super::*;
use std::time::Duration;

#[derive(Default)]
struct App {
    settings: bool,
    nested: bool,
    enabled: bool,
    mode: Option<u32>,
    chosen: Vec<usize>,
    closed: Vec<CloseReason>,
    nested_closed: Vec<CloseReason>,
}

impl App {
    fn build(&mut self, context: &mut Context) {
        Window::new("Main").show(context, |ui| {
            if ui.button("Open").clicked() {
                self.settings = true;
            }
            ui.button("Other");
            let dialog = Dialog::new("settings", "Settings")
                .description("Change how it works")
                .action(DialogAction::new("Cancel"))
                .action(DialogAction::new("Save").primary());
            let output = dialog.show(ui, &mut self.settings, |ui| {
                ui.checkbox(&mut self.enabled, "Enabled");
                let modes = [(1, "Fast"), (2, "Exact")];
                ui.add(
                    ComboBox::from_pairs(&mut self.mode, modes)
                        .id_source("mode")
                        .label("Mode"),
                );
                if ui.button("More").clicked() {
                    self.nested = true;
                }
                let inner = Modal::new("more").accessible_label("Advanced");
                let shown = inner.show(ui, &mut self.nested, |ui| {
                    ui.button("Reset");
                });
                self.nested_closed
                    .extend(shown.and_then(|output| output.closed));
            });
            if let Some(output) = output {
                self.chosen.extend(output.action);
                self.closed.extend(output.closed);
            }
        });
    }
}

fn opened() -> (Harness, App) {
    let mut harness = still();
    let mut app = App::default();
    harness.pass(|ctx| app.build(ctx));
    let open = harness.tree.expect(Role::Button, "Open");
    assert!(harness.act(open, Action::Focus));
    assert!(harness.act(open, Action::Click));
    harness.settle(|ctx| app.build(ctx));
    (harness, app)
}

#[test]
fn an_open_dialog_is_named_described_and_replaces_what_is_behind_it() {
    let (mut harness, mut app) = opened();
    let tree = &harness.tree;
    let dialog = tree.expect(Role::Dialog, "Settings");
    let node = tree.node(dialog);
    assert!(node.is_modal());
    assert_eq!(node.description(), Some("Change how it works"));
    let title = tree.expect(Role::Heading, "Settings");
    assert_eq!(node.labelled_by(), [title]);
    assert_eq!(tree.parent(dialog), Some(tree.root()));
    assert!(inside(tree, title, dialog));
    for (role, name) in [
        (Role::Label, "Change how it works"),
        (Role::CheckBox, "Enabled"),
        (Role::ComboBox, "Mode"),
        (Role::Button, "More"),
        (Role::Button, "Cancel"),
        (Role::Button, "Save"),
        (Role::Button, "Close"),
    ] {
        assert!(inside(tree, tree.expect(role, name), dialog), "{name}");
    }
    assert!(
        tree.find(Role::Window, "Main").is_none(),
        "the window behind is out of the tree"
    );
    assert!(tree.find(Role::Button, "Open").is_none());
    assert!(tree.find(Role::Button, "Other").is_none());
    assert!(
        inside(tree, tree.focus(), dialog),
        "focus is inside the dialog"
    );
    assert_ne!(tree.focus(), dialog, "its first control took focus");
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    assert!(harness.context.diagnostics().is_empty());
}

#[test]
fn tab_keeps_focus_inside_the_dialog() {
    let (mut harness, mut app) = opened();
    let dialog = harness.tree.expect(Role::Dialog, "Settings");
    let mut seen = std::collections::HashSet::new();
    for _ in 0..12 {
        key(&mut harness.context, KeyCode::Tab);
        harness.pass(|ctx| app.build(ctx));
        let focus = harness.tree.focus();
        assert!(inside(&harness.tree, focus, dialog) && focus != dialog);
        seen.insert(harness.tree.name(focus));
    }
    for name in ["Enabled", "Mode", "More", "Cancel", "Save", "Close"] {
        assert!(seen.contains(name), "{name} was never focused: {seen:?}");
    }
    assert!(app.settings);
}

#[test]
fn an_action_closes_once_and_restores_the_background_and_focus() {
    let (mut harness, mut app) = opened();
    let save = harness.tree.expect(Role::Button, "Save");
    assert!(harness.act(save, Action::Click));
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(
        (app.chosen.as_slice(), app.closed.as_slice()),
        ([1].as_slice(), [CloseReason::Action].as_slice())
    );
    assert!(!app.settings);
    let tree = &harness.tree;
    assert!(tree.all(Role::Dialog).is_empty());
    assert!(tree.find(Role::Button, "Save").is_none());
    let open = tree.expect(Role::Button, "Open");
    assert!(inside(tree, open, tree.expect(Role::Window, "Main")));
    assert_eq!(tree.focus(), open, "focus returns to what had it");
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
}

#[test]
fn the_close_button_and_escape_close_with_their_reasons() {
    let (mut harness, mut app) = opened();
    let close = harness.tree.expect(Role::Button, "Close");
    assert!(harness.act(close, Action::Click));
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(app.closed, [CloseReason::CloseButton]);
    assert!(harness.tree.all(Role::Dialog).is_empty());

    let open = harness.tree.expect(Role::Button, "Open");
    harness.act(open, Action::Click);
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(harness.tree.all(Role::Dialog).len(), 1);
    key(&mut harness.context, KeyCode::Escape);
    harness.pass(|ctx| app.build(ctx));
    assert!(
        harness.tree.all(Role::Dialog).is_empty(),
        "gone with the pass that closed it"
    );
    assert!(harness.tree.find(Role::Button, "Other").is_some());
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(app.closed, [CloseReason::CloseButton, CloseReason::Escape]);
    assert!(app.chosen.is_empty());
}

#[test]
fn only_the_top_of_two_stacked_modals_is_in_the_tree() {
    let (mut harness, mut app) = opened();
    let more = harness.tree.expect(Role::Button, "More");
    harness.act(more, Action::Focus);
    harness.act(more, Action::Click);
    harness.settle(|ctx| app.build(ctx));
    let tree = &harness.tree;
    let advanced = tree.expect(Role::Dialog, "Advanced");
    assert!(tree.node(advanced).is_modal());
    assert!(tree.find(Role::Dialog, "Settings").is_none());
    assert!(tree.find(Role::Button, "Save").is_none());
    assert!(tree.find(Role::Button, "Open").is_none());
    assert!(inside(tree, tree.expect(Role::Button, "Reset"), advanced));
    assert!(inside(tree, tree.focus(), advanced));
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    for _ in 0..6 {
        key(&mut harness.context, KeyCode::Tab);
        harness.pass(|ctx| app.build(ctx));
        assert!(inside(&harness.tree, harness.tree.focus(), advanced));
    }
    // Closing the top one brings the one under it back, with its focus.
    let close = harness
        .tree
        .all(Role::Button)
        .into_iter()
        .find(|id| harness.tree.name(*id) == "Close")
        .expect("the corner button");
    harness.act(close, Action::Click);
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(app.nested_closed, [CloseReason::CloseButton]);
    let tree = &harness.tree;
    let settings = tree.expect(Role::Dialog, "Settings");
    assert!(tree.all(Role::Dialog).len() == 1 && tree.find(Role::Button, "Open").is_none());
    assert_eq!(tree.focus(), tree.expect(Role::Button, "More"));
    assert!(inside(tree, tree.focus(), settings));
    assert!(app.settings && app.closed.is_empty());
}

#[test]
fn a_popup_opened_inside_a_modal_is_in_the_tree() {
    let (mut harness, mut app) = opened();
    let mode = harness.tree.expect(Role::ComboBox, "Mode");
    assert!(harness.act(mode, Action::Expand));
    harness.settle(|ctx| app.build(ctx));
    let tree = &harness.tree;
    let dialog = tree.expect(Role::Dialog, "Settings");
    let list = tree.expect(Role::ListBox, "Mode");
    assert!(!inside(tree, list, dialog));
    let layers = tree.node(tree.root()).children();
    let position = |of: NodeId| layers.iter().position(|layer| inside(tree, of, *layer));
    assert!(
        position(list) > position(dialog),
        "the popup is above its dialog"
    );
    assert_eq!(names(tree, Role::ListBoxOption), ["Fast", "Exact"]);
    assert_eq!(tree.focus(), mode);
    assert_eq!(harness.pass(|ctx| app.build(ctx)), None);
    let exact = harness.tree.expect(Role::ListBoxOption, "Exact");
    harness.act(exact, Action::Click);
    harness.settle(|ctx| app.build(ctx));
    assert_eq!(app.mode, Some(2));
    assert!(harness.tree.all(Role::ListBox).is_empty());
    assert!(harness.tree.find(Role::Dialog, "Settings").is_some() && app.settings);
}

#[test]
fn a_fading_dialog_is_already_out_of_the_tree() {
    let mut harness = Harness::new();
    let mut app = App::default();
    let mut spoken = Spoken::default();
    let start = Instant::now();
    let mut at = |harness: &mut Harness, app: &mut App, ms: u64| {
        let now = start + Duration::from_millis(ms);
        pass_at(harness, now, &mut spoken, |ctx| app.build(ctx))
    };
    at(&mut harness, &mut app, 0);
    let open = harness.tree.expect(Role::Button, "Open");
    harness.act(open, Action::Click);
    for ms in (16..1500).step_by(16) {
        at(&mut harness, &mut app, ms);
    }
    assert!(harness.tree.find(Role::Dialog, "Settings").is_some());
    assert_eq!(
        at(&mut harness, &mut app, 2000),
        None,
        "the opening animation has settled"
    );
    let cancel = harness.tree.expect(Role::Button, "Cancel");
    harness.act(cancel, Action::Click);
    for ms in (2016..4000).step_by(16) {
        at(&mut harness, &mut app, ms);
        if !app.settings {
            assert!(harness.tree.all(Role::Dialog).is_empty(), "at {ms} ms");
            assert!(
                harness.tree.find(Role::Button, "Cancel").is_none(),
                "at {ms} ms"
            );
            assert!(
                harness.tree.find(Role::Button, "Open").is_some(),
                "at {ms} ms"
            );
        }
    }
    assert_eq!(app.chosen, [0]);
    assert_eq!(at(&mut harness, &mut app, 5000), None);
}
