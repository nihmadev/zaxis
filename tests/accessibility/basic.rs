use crate::support::*;

#[derive(Default)]
struct Model {
    checked: bool,
    volume: f32,
    saves: u32,
    extra: bool,
}

fn build(context: &mut Context, model: &mut Model) {
    Window::new("Settings").show(context, |ui| {
        ui.heading("Audio");
        ui.label("Output device");
        if ui.button("Save").clicked() {
            model.saves += 1;
        }
        ui.checkbox(&mut model.checked, "Mute");
        ui.add(
            Slider::new(&mut model.volume, 0.0..=10.0)
                .step(1.0)
                .text("Volume"),
        );
        if model.extra {
            ui.button("Extra");
        }
    });
}

#[test]
fn a_window_publishes_roles_names_and_order() {
    let mut harness = Harness::new();
    let mut model = Model::default();
    assert!(harness.pass(|ctx| build(ctx, &mut model)).is_some());
    let tree = &harness.tree;
    assert_eq!(tree.node(tree.root()).role(), Role::Window);
    let window = tree.expect(Role::Window, "Settings");
    assert_eq!(tree.parent(window), Some(tree.root()));
    let order: Vec<_> = tree
        .node(window)
        .children()
        .iter()
        .map(|id| (tree.node(*id).role(), tree.name(*id)))
        .collect();
    assert_eq!(
        order,
        [
            (Role::Heading, "Audio".to_owned()),
            (Role::Label, "Output device".to_owned()),
            (Role::Button, "Save".to_owned()),
            (Role::CheckBox, "Mute".to_owned()),
            (Role::Slider, "Volume".to_owned()),
        ]
    );
    let slider = harness.node(Role::Slider, "Volume");
    assert_eq!(slider.numeric_value(), Some(0.0));
    assert_eq!(slider.max_numeric_value(), Some(10.0));
    assert_eq!(slider.numeric_value_step(), Some(1.0));
    assert_eq!(
        harness.node(Role::CheckBox, "Mute").toggled(),
        Some(Toggled::False)
    );
}

#[test]
fn an_unchanged_pass_publishes_nothing_and_a_change_publishes_one_node() {
    let mut harness = Harness::new();
    let mut model = Model::default();
    harness.pass(|ctx| build(ctx, &mut model));
    assert_eq!(harness.pass(|ctx| build(ctx, &mut model)), None);
    model.checked = true;
    assert_eq!(harness.pass(|ctx| build(ctx, &mut model)), Some(1));
    assert_eq!(
        harness.node(Role::CheckBox, "Mute").toggled(),
        Some(Toggled::True)
    );
    // A widget that appears is sent together with its parent's new child list; when it
    // goes away only the parent is.
    model.extra = true;
    assert_eq!(harness.pass(|ctx| build(ctx, &mut model)), Some(2));
    assert!(harness.tree.find(Role::Button, "Extra").is_some());
    model.extra = false;
    assert_eq!(harness.pass(|ctx| build(ctx, &mut model)), Some(1));
    assert!(harness.tree.find(Role::Button, "Extra").is_none());
}

#[test]
fn bounds_are_physical_pixels_at_every_scale() {
    let mut reference = None;
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut harness = Harness::with_scale(scale);
        let mut model = Model::default();
        harness.pass(|ctx| build(ctx, &mut model));
        let bounds = harness.node(Role::Button, "Save").bounds().expect("bounds");
        let logical = [bounds.x0, bounds.y0, bounds.x1, bounds.y1].map(|v| v / scale);
        let expected = *reference.get_or_insert(logical);
        for (got, want) in logical.iter().zip(expected) {
            assert!(
                (got - want).abs() <= 1.0,
                "scale {scale}: {logical:?} vs {expected:?}"
            );
        }
        let root = harness
            .tree
            .node(harness.tree.root())
            .bounds()
            .expect("root bounds");
        assert_eq!((root.x1, root.y1), (800.0 * scale, 600.0 * scale));
    }
}

#[test]
fn click_toggle_and_range_requests_act_like_input() {
    let mut harness = Harness::new();
    let mut model = Model::default();
    harness.pass(|ctx| build(ctx, &mut model));
    let save = harness.tree.expect(Role::Button, "Save");
    assert!(harness.act(save, Action::Click));
    harness.settle(|ctx| build(ctx, &mut model));
    assert_eq!(model.saves, 1, "one request is one click");

    let mute = harness.tree.expect(Role::CheckBox, "Mute");
    assert!(harness.act(mute, Action::Click));
    harness.settle(|ctx| build(ctx, &mut model));
    assert!(model.checked);

    let volume = harness.tree.expect(Role::Slider, "Volume");
    assert!(harness.act(volume, Action::Increment));
    harness.settle(|ctx| build(ctx, &mut model));
    assert_eq!(model.volume, 1.0);
    assert!(harness.act_with(volume, Action::SetValue, ActionData::NumericValue(7.4)));
    harness.settle(|ctx| build(ctx, &mut model));
    assert_eq!(
        model.volume, 7.0,
        "the step applies to a value set from outside"
    );
    for value in [f64::NAN, f64::INFINITY] {
        harness.act_with(volume, Action::SetValue, ActionData::NumericValue(value));
        harness.settle(|ctx| build(ctx, &mut model));
        assert_eq!(model.volume, 7.0);
    }
    harness.act_with(volume, Action::SetValue, ActionData::NumericValue(99.0));
    harness.settle(|ctx| build(ctx, &mut model));
    assert_eq!(model.volume, 10.0);
    assert!(harness.act(volume, Action::Decrement));
    harness.settle(|ctx| build(ctx, &mut model));
    assert_eq!(model.volume, 9.0);
    assert_eq!(
        harness.node(Role::Slider, "Volume").numeric_value(),
        Some(9.0)
    );
}

#[test]
fn focus_follows_the_keyboard_and_focus_requests() {
    let mut harness = Harness::new();
    let mut model = Model::default();
    harness.pass(|ctx| build(ctx, &mut model));
    assert_eq!(harness.tree.focus(), harness.tree.root());
    harness
        .context
        .key(KeyCode::Tab, ElementState::Pressed, false);
    harness.pass(|ctx| build(ctx, &mut model));
    assert_eq!(
        harness.tree.focus(),
        harness.tree.expect(Role::Button, "Save")
    );
    let mute = harness.tree.expect(Role::CheckBox, "Mute");
    assert!(harness.act(mute, Action::Focus));
    assert_eq!(
        harness.pass(|ctx| build(ctx, &mut model)),
        Some(0),
        "focus alone moved"
    );
    assert_eq!(harness.tree.focus(), mute);
    assert_eq!(harness.pass(|ctx| build(ctx, &mut model)), None);
}

#[test]
fn requests_for_unknown_or_disabled_nodes_are_ignored() {
    let mut harness = Harness::new();
    let mut model = Model::default();
    let disabled = |ctx: &mut Context, model: &mut Model| {
        Window::new("Settings").show(ctx, |ui| {
            ui.add_enabled_ui(false, |ui| {
                if ui.button("Save").clicked() {
                    model.saves += 1;
                }
            });
        });
    };
    harness.pass(|ctx| disabled(ctx, &mut model));
    let save = harness.tree.expect(Role::Button, "Save");
    assert!(harness.tree.node(save).is_disabled());
    assert!(!harness.act(save, Action::Click));
    assert!(!harness.act(NodeId(0xdead_beef), Action::Click));
    assert!(!harness.act(harness.tree.root(), Action::Click));
    harness.settle(|ctx| disabled(ctx, &mut model));
    assert_eq!(model.saves, 0);
}

#[test]
fn deactivation_forgets_the_tree_and_reactivation_sends_all_of_it() {
    let mut harness = Harness::new();
    let mut model = Model::default();
    let full = harness.pass(|ctx| build(ctx, &mut model));
    harness.context.set_accessibility_active(false);
    harness.context.run(|ctx| build(ctx, &mut model));
    assert!(harness.context.take_accessibility_update().is_none());
    let passes = harness.context.accessibility_stats().passes;
    harness.context.run(|ctx| build(ctx, &mut model));
    assert_eq!(harness.context.accessibility_stats().passes, passes);
    harness.context.set_accessibility_active(true);
    harness.context.run(|ctx| build(ctx, &mut model));
    let update = harness
        .context
        .take_accessibility_update()
        .expect("a full tree");
    assert!(update.tree.is_some());
    assert_eq!(Some(update.nodes.len()), full);
}

#[test]
fn a_control_without_a_name_is_reported() {
    let mut harness = Harness::new();
    let mut on = false;
    let mut build = |ctx: &mut Context, named: bool| {
        Window::new("Test").show(ctx, |ui| {
            if named {
                ui.add(Checkbox::new(&mut on, "").accessible_label("Dark mode"));
            } else {
                ui.add(Checkbox::new(&mut on, ""));
            }
        });
    };
    harness.pass(|ctx| build(ctx, false));
    assert!(harness
        .context
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::MissingAccessibleName));
    harness.pass(|ctx| build(ctx, true));
    assert!(harness.context.diagnostics().is_empty());
    assert!(harness.tree.find(Role::CheckBox, "Dark mode").is_some());
}
