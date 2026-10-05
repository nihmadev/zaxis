use super::*;

fn window(context: &mut Context, build: impl FnOnce(&mut Ui<'_>)) {
    Window::new("Main").show(context, |ui| {
        ui.button("Behind");
        build(ui);
    });
}

fn nameless(harness: &Harness) -> usize {
    let diagnostics = harness.context.diagnostics();
    let nameless = diagnostics.iter();
    nameless
        .filter(|d| d.kind == DiagnosticKind::MissingAccessibleName)
        .count()
}

#[test]
fn a_confirmation_is_an_alert_dialog_that_answers_once() {
    let mut harness = still();
    let mut open = true;
    let mut answers = Vec::new();
    let build = |ctx: &mut Context, open: &mut bool, answers: &mut Vec<Confirmation>| {
        window(ctx, |ui| {
            let confirm = Confirm::new("delete")
                .title("Delete file?")
                .description("This cannot be undone.")
                .confirm_label("Delete")
                .danger();
            answers.extend(confirm.show(ui, open));
        });
    };
    harness.settle(|ctx| build(ctx, &mut open, &mut answers));
    let tree = &harness.tree;
    let dialog = tree.expect(Role::AlertDialog, "Delete file?");
    assert!(tree.node(dialog).is_modal());
    assert_eq!(
        tree.node(dialog).description(),
        Some("This cannot be undone.")
    );
    assert!(tree.find(Role::Button, "Behind").is_none());
    assert!(
        tree.find(Role::Button, "Close").is_none(),
        "a confirmation has no corner button"
    );
    let cancel = tree.expect(Role::Button, "Cancel");
    assert_eq!(tree.focus(), cancel, "focus starts on the safe choice");
    assert_eq!(
        harness.pass(|ctx| build(ctx, &mut open, &mut answers)),
        None
    );
    let delete = harness.tree.expect(Role::Button, "Delete");
    assert!(harness.act(delete, Action::Click));
    harness.settle(|ctx| build(ctx, &mut open, &mut answers));
    assert_eq!(answers, [Confirmation::Confirmed]);
    assert!(!open && harness.tree.all(Role::AlertDialog).is_empty());
    assert!(harness.tree.find(Role::Button, "Behind").is_some());

    open = true;
    harness.settle(|ctx| build(ctx, &mut open, &mut answers));
    let cancel = harness.tree.expect(Role::Button, "Cancel");
    harness.act(cancel, Action::Click);
    harness.settle(|ctx| build(ctx, &mut open, &mut answers));
    assert_eq!(
        answers,
        [
            Confirmation::Confirmed,
            Confirmation::Cancelled(CloseReason::Action)
        ]
    );
}

#[test]
fn a_confirmation_without_a_title_is_named_by_its_message() {
    let mut harness = still();
    let mut open = true;
    let mut build = |ctx: &mut Context| {
        window(ctx, |ui| {
            Confirm::new("quit")
                .description("Quit without saving?")
                .show(ui, &mut open);
        });
    };
    harness.settle(&mut build);
    let dialog = harness.node(Role::AlertDialog, "Quit without saving?");
    assert_eq!(dialog.description(), None, "the message is not repeated");
    assert_eq!(nameless(&harness), 0);
}

#[test]
fn a_bare_modal_takes_its_name_from_its_header_or_from_the_caller() {
    #[derive(Clone, Copy, PartialEq)]
    enum Name {
        None,
        Header,
        Given,
    }
    let mut harness = still();
    let mut open = true;
    let mut build = |ctx: &mut Context, name: Name| {
        window(ctx, |ui| {
            let mut modal = Modal::new("panel").close_button(false);
            if name == Name::Given {
                modal = modal.accessible_label("Export options");
            }
            if name == Name::Header {
                modal.show_parts(
                    ui,
                    &mut open,
                    |ui| {
                        ui.heading("Export");
                    },
                    |ui| {
                        ui.label("Body text");
                    },
                    |ui| {
                        ui.button("Done");
                    },
                );
            } else {
                modal.show(ui, &mut open, |ui| {
                    ui.label("Body text");
                });
            }
        });
    };
    harness.settle(|ctx| build(ctx, Name::None));
    assert_eq!(harness.tree.all(Role::Dialog).len(), 1);
    assert_eq!(nameless(&harness), 1, "a dialog nobody named is reported");
    // Nothing in it takes focus: the dialog itself has it.
    assert_eq!(harness.tree.focus(), harness.tree.all(Role::Dialog)[0]);

    harness.settle(|ctx| build(ctx, Name::Given));
    assert!(harness.tree.find(Role::Dialog, "Export options").is_some());
    assert_eq!(nameless(&harness), 0);

    harness.settle(|ctx| build(ctx, Name::Header));
    let dialog = harness.tree.expect(Role::Dialog, "Export");
    assert_eq!(harness.tree.node(dialog).description(), None);
    assert_eq!(nameless(&harness), 0);
    assert_eq!(
        harness.tree.focus(),
        harness.tree.expect(Role::Button, "Done")
    );
}

#[test]
fn a_named_dialog_keeps_the_name_it_was_given() {
    let mut harness = still();
    let mut open = true;
    let mut build = |ctx: &mut Context| {
        window(ctx, |ui| {
            Dialog::new("about", "About")
                .modal(|modal| modal.accessible_label("About this application"))
                .action(DialogAction::new("OK"))
                .show(ui, &mut open, |_| {});
        });
    };
    harness.settle(&mut build);
    let dialog = harness.node(Role::Dialog, "About this application");
    assert!(dialog.labelled_by().is_empty());
    assert_eq!(dialog.description(), None);
    assert_eq!(nameless(&harness), 0);
}

#[test]
fn a_modal_takes_down_a_tooltip_that_was_asked_for_behind_it() {
    let mut harness = still();
    let mut open = false;
    let build = |ctx: &mut Context, open: &mut bool| {
        Window::new("Main").show(ctx, |ui| {
            ui.add(Button::new("Save").tooltip("Write the file"));
            Modal::new("m")
                .accessible_label("Notice")
                .show(ui, open, |ui| {
                    ui.button("Fine");
                });
        });
    };
    harness.pass(|ctx| build(ctx, &mut open));
    let save = harness.tree.expect(Role::Button, "Save");
    assert!(harness.act(save, Action::ShowTooltip));
    harness.settle(|ctx| build(ctx, &mut open));
    assert_eq!(harness.tree.all(Role::Tooltip).len(), 1);
    open = true;
    harness.settle(|ctx| build(ctx, &mut open));
    assert!(harness.tree.find(Role::Dialog, "Notice").is_some());
    assert!(harness.tree.all(Role::Tooltip).is_empty());
    open = false;
    harness.settle(|ctx| build(ctx, &mut open));
    assert!(
        harness.tree.all(Role::Tooltip).is_empty(),
        "it does not come back"
    );
    assert!(harness.tree.find(Role::Button, "Save").is_some());
}
