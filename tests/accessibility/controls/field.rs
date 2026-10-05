use super::platform_names;
use crate::support::*;
use zaxis::accesskit::Live;

fn form(ctx: &mut Context, name: &mut String, validation: Validation) {
    Window::new("Test").show(ctx, |ui| {
        Field::new("Name")
            .hint("As on your passport")
            .validation(validation)
            .show(ui, |ui| {
                ui.text_edit(name);
                ui.button("Clear");
            });
    });
}

#[test]
fn a_field_names_and_describes_its_first_control() {
    let mut harness = Harness::new();
    let mut name = String::new();
    harness.pass(|ctx| form(ctx, &mut name, Validation::ok()));
    let label = harness.tree.expect(Role::Label, "Name");
    let hint = harness.tree.expect(Role::Label, "As on your passport");
    let input = harness.tree.expect(Role::TextInput, "Name");
    let node = harness.tree.node(input);
    assert_eq!(node.labelled_by(), [label]);
    assert_eq!(node.described_by(), [hint]);
    assert_eq!(node.description(), Some("As on your passport"));
    assert_eq!(node.error_message(), None);
    assert!(node.invalid().is_none());
    assert_eq!(
        harness.tree.node(hint).live(),
        None,
        "a hint is not announced"
    );
    let clear = harness.tree.expect(Role::Button, "Clear");
    assert!(
        harness.tree.node(clear).labelled_by().is_empty(),
        "only the first control"
    );
    assert!(harness.context.diagnostics().is_empty());
    assert_eq!(
        harness.pass(|ctx| form(ctx, &mut name, Validation::ok())),
        None
    );
}

#[test]
fn a_validation_error_marks_the_control_and_is_a_live_message() {
    let mut harness = Harness::new();
    let mut name = String::new();
    harness.pass(|ctx| form(ctx, &mut name, Validation::ok()));
    let input = harness.tree.expect(Role::TextInput, "Name");

    let error = || Validation::error("A name is required");
    let sent = harness.pass(|ctx| form(ctx, &mut name, error()));
    assert!(sent.is_some(), "the message that appeared is published");
    let message = harness.tree.expect(Role::Label, "A name is required");
    assert_eq!(harness.tree.node(message).live(), Some(Live::Assertive));
    let update = harness.tree.last.as_ref().expect("an update");
    assert!(
        update.nodes.iter().any(|(id, _)| *id == message),
        "the live message is a new node of this update"
    );
    let node = harness.tree.node(input);
    assert!(node.invalid().is_some());
    assert_eq!(node.error_message(), Some(message));
    assert_eq!(node.described_by(), [message]);
    assert_eq!(node.description(), Some("A name is required"));
    assert!(harness
        .tree
        .find(Role::Label, "As on your passport")
        .is_none());
    harness.settle(|ctx| form(ctx, &mut name, error()));
    assert_eq!(
        harness.pass(|ctx| form(ctx, &mut name, error())),
        None,
        "a standing error is announced once"
    );

    // A warning is polite and is no error of the control.
    let warning = || Validation::warning("Looks short");
    harness.settle(|ctx| form(ctx, &mut name, warning()));
    let message = harness.tree.expect(Role::Label, "Looks short");
    assert_eq!(harness.tree.node(message).live(), Some(Live::Polite));
    let node = harness.tree.node(input);
    assert!(node.invalid().is_none());
    assert_eq!(node.error_message(), None);
    assert_eq!(node.described_by(), [message]);

    // An error status without a message still marks the control.
    harness.settle(|ctx| form(ctx, &mut name, SemanticStatus::Error.into()));
    let node = harness.tree.node(input);
    assert!(node.invalid().is_some());
    assert_eq!(node.error_message(), None);
    assert_eq!(node.description(), Some("As on your passport"));
}

#[test]
fn a_field_labels_whatever_control_it_holds() {
    let mut harness = Harness::new();
    let (mut volume, mut on, mut mode, mut color) = (3.0_f64, false, 0_u8, Color::rgb(1, 2, 3));
    let mut build = |ctx: &mut Context| {
        Window::new("Test").show(ctx, |ui| {
            Field::new("Volume").show(ui, |ui| ui.add(NumberInput::new(&mut volume)));
            Field::new("Network").show(ui, |ui| ui.switch(&mut on, "Wi-Fi"));
            Field::new("Mode").show(ui, |ui| {
                ui.radio_group(&mut mode, [(0, "Auto"), (1, "Manual")])
            });
            Field::new("Tint").show(ui, |ui| ui.add(ColorPicker::new(&mut color, "")));
        });
    };
    harness.pass(&mut build);
    assert!(
        harness.context.diagnostics().is_empty(),
        "{:?}",
        harness.context.diagnostics()
    );
    assert!(harness.tree.find(Role::SpinButton, "Volume").is_some());
    assert_eq!(
        platform_names(&harness, Role::TextInput),
        [Some("Volume".to_owned())],
        "the text field of the number takes the label"
    );
    // A control with a caption of its own keeps it as its name.
    let switch = harness.tree.expect(Role::Switch, "Wi-Fi");
    let network = harness.tree.expect(Role::Label, "Network");
    assert_eq!(harness.tree.node(switch).labelled_by(), [network]);
    assert!(harness.tree.find(Role::RadioGroup, "Mode").is_some());
    assert!(harness.tree.find(Role::ColorWell, "Tint").is_some());
    assert_eq!(harness.pass(&mut build), None);
}
