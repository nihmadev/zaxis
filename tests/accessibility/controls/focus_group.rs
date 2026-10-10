use crate::support::*;

struct Model {
    saves: u32,
    present: bool,
}

fn build(ctx: &mut Context, m: &mut Model) {
    Root::new().show(ctx, |ui| {
        ui.button("Outside");
        if m.present {
            FocusGroup::new("tools")
                .label("Tools")
                .role(AccessRole::Toolbar)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            m.saves += 1;
                        }
                        ui.button("Open");
                        ui.add_enabled_ui(false, |ui| ui.button("Print"));
                        ui.button("Close");
                    });
                });
        }
    });
}

#[test]
fn a_group_is_a_named_node_of_the_role_it_was_given_and_its_members_describe_themselves() {
    let mut h = Harness::new();
    let mut m = Model {
        saves: 0,
        present: true,
    };
    h.pass(|c| build(c, &mut m));
    let toolbar = h.tree.expect(Role::Toolbar, "Tools");
    let children = h.tree.node(toolbar).children().to_vec();
    let names: Vec<_> = children.iter().map(|id| h.tree.name(*id)).collect();
    assert_eq!(names, ["Save", "Open", "Print", "Close"]);
    assert!(children
        .iter()
        .all(|id| h.tree.node(*id).role() == Role::Button));
    assert!(h
        .tree
        .node(h.tree.expect(Role::Button, "Print"))
        .is_disabled());
}

#[test]
fn assistive_technology_can_focus_any_member_and_it_becomes_the_stop() {
    let mut h = Harness::new();
    let mut m = Model {
        saves: 0,
        present: true,
    };
    h.pass(|c| build(c, &mut m));
    let close = h.tree.expect(Role::Button, "Close");
    assert!(
        h.act(close, Action::Focus),
        "a member that is not the stop takes focus"
    );
    h.pass(|c| build(c, &mut m));
    assert_eq!(h.tree.focus(), close);
    h.context.key(KeyCode::Tab, ElementState::Pressed, false);
    h.pass(|c| build(c, &mut m));
    assert_ne!(h.tree.focus(), close, "Tab left the group");
    h.context
        .set_modifiers(winit::keyboard::ModifiersState::SHIFT);
    h.context.key(KeyCode::Tab, ElementState::Pressed, false);
    h.context
        .set_modifiers(winit::keyboard::ModifiersState::empty());
    h.pass(|c| build(c, &mut m));
    assert_eq!(h.tree.focus(), close, "and Shift+Tab returns to it");
}

#[test]
fn a_click_request_activates_a_member_once() {
    let mut h = Harness::new();
    let mut m = Model {
        saves: 0,
        present: true,
    };
    h.pass(|c| build(c, &mut m));
    let save = h.tree.expect(Role::Button, "Save");
    assert!(h.act(save, Action::Click));
    h.settle(|c| build(c, &mut m));
    assert_eq!(m.saves, 1);
}

#[test]
fn keyboard_navigation_moves_the_focused_node_with_it() {
    let mut h = Harness::new();
    let mut m = Model {
        saves: 0,
        present: true,
    };
    h.pass(|c| build(c, &mut m));
    h.context.key(KeyCode::Tab, ElementState::Pressed, false);
    h.context.key(KeyCode::Tab, ElementState::Pressed, false);
    h.pass(|c| build(c, &mut m));
    assert_eq!(h.tree.focus(), h.tree.expect(Role::Button, "Save"));
    h.context
        .key(KeyCode::ArrowRight, ElementState::Pressed, false);
    h.context
        .key(KeyCode::ArrowRight, ElementState::Pressed, false);
    h.pass(|c| build(c, &mut m));
    assert_eq!(
        h.tree.focus(),
        h.tree.expect(Role::Button, "Close"),
        "Print is disabled and skipped"
    );
}

#[test]
fn a_group_that_disappears_leaves_no_focus_on_a_removed_node() {
    let mut h = Harness::new();
    let mut m = Model {
        saves: 0,
        present: true,
    };
    h.pass(|c| build(c, &mut m));
    let open = h.tree.expect(Role::Button, "Open");
    h.act(open, Action::Focus);
    h.pass(|c| build(c, &mut m));
    assert_eq!(h.tree.focus(), open);
    m.present = false;
    h.settle(|c| build(c, &mut m));
    assert_eq!(h.tree.focus(), h.tree.root(), "focus fell back to the root");
    assert!(h.tree.all(Role::Toolbar).is_empty());
    assert_eq!(h.context.input_stats().focus_groups, 0);
}

#[test]
fn radio_buttons_are_all_focusable_for_assistive_technology() {
    #[derive(Hash, PartialEq, Clone, Copy)]
    enum Pick {
        A,
        B,
        C,
    }
    let mut h = Harness::new();
    let mut pick = Pick::A;
    let run = |c: &mut Context, pick: &mut Pick| {
        Root::new().show(c, |ui| {
            ui.radio_group(
                pick,
                [(Pick::A, "Alpha"), (Pick::B, "Beta"), (Pick::C, "Gamma")],
            );
        })
    };
    h.pass(|c| run(c, &mut pick));
    let gamma = h.tree.expect(Role::RadioButton, "Gamma");
    assert!(h.act(gamma, Action::Focus));
    h.settle(|c| run(c, &mut pick));
    assert_eq!(h.tree.focus(), gamma);
    assert!(pick == Pick::A, "focus alone selects nothing");
}
