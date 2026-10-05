//! Custom widgets, call-site overrides, announcements and the tree's own guarantees.
use crate::support::*;
use std::time::Duration;

/// A dial drawn by the application: focusable, draggable, described by hand.
fn dial(ui: &mut Ui<'_>, volume: &mut f64, requests: &mut Vec<AccessAction>) -> Response {
    let rect = ui.allocate_space(vec2(48.0, 48.0));
    let response = ui.interact(rect, "dial", Sense::DRAG | Sense::FOCUS);
    for request in ui.accessible(&response, |node| {
        node.role(AccessRole::Slider)
            .label("Volume")
            .numeric(*volume, 0.0, 1.0)
            .step(0.25)
            .action(AccessActionKind::Increment)
            .action(AccessActionKind::Decrement)
            .action(AccessActionKind::SetValue);
    }) {
        match &request {
            AccessAction::Increment => *volume = (*volume + 0.25).min(1.0),
            AccessAction::Decrement => *volume = (*volume - 0.25).max(0.0),
            AccessAction::SetNumericValue(value) if value.is_finite() => {
                *volume = value.clamp(0.0, 1.0)
            }
            _ => {}
        }
        requests.push(request);
    }
    response
}

#[test]
fn a_custom_widget_describes_itself_and_receives_requests() {
    let mut harness = Harness::new();
    let (mut volume, mut requests, mut clicks) = (0.5, Vec::new(), 0);
    let mut build = |ctx: &mut Context| {
        Window::new("Mixer").show(ctx, |ui| {
            dial(ui, &mut volume, &mut requests);
            let rect = ui.allocate_space(vec2(80.0, 24.0));
            let pad = ui.interact(rect, "pad", Sense::CLICK | Sense::FOCUS);
            ui.accessible(&pad, |node| {
                node.role(AccessRole::Button).label("Pad");
            });
            clicks += u32::from(pad.clicked());
            // Custom geometry nobody described is not in the tree at all.
            let rect = ui.allocate_space(vec2(80.0, 24.0));
            ui.interact(rect, "silent", Sense::CLICK);
        });
    };
    harness.pass(&mut build);
    let window = harness.tree.expect(Role::Window, "Mixer");
    assert_eq!(harness.tree.node(window).children().len(), 2);
    let slider = harness.tree.expect(Role::Slider, "Volume");
    assert_eq!(harness.tree.node(slider).numeric_value(), Some(0.5));
    assert!(
        harness.tree.node(slider).supports_action(Action::Focus),
        "its region takes focus"
    );
    assert!(
        !harness.tree.node(slider).supports_action(Action::Click),
        "and no clicks"
    );
    assert!(harness.act(slider, Action::Increment));
    assert!(harness.act_with(slider, Action::SetValue, ActionData::NumericValue(f64::NAN)));
    assert!(
        !harness.act(slider, Action::Expand),
        "an action it never declared"
    );
    harness.settle(&mut build);
    let pad = harness.tree.expect(Role::Button, "Pad");
    assert!(harness.tree.node(pad).supports_action(Action::Click));
    assert!(harness.act(pad, Action::Click));
    harness.settle(&mut build);
    assert_eq!(harness.tree.node(slider).numeric_value(), Some(0.75));
    drop(build);
    assert_eq!(volume, 0.75);
    assert_eq!(requests.len(), 2, "each request is delivered once");
    assert_eq!(clicks, 1, "a click request is an ordinary click");
}

#[test]
fn call_site_overrides_name_describe_and_hide() {
    let mut harness = Harness::new();
    let mut on = false;
    harness.pass(|ctx| {
        Window::new("Test").show(ctx, |ui| {
            let save = Button::new("##save").accessible_label("Save document");
            ui.add(save.accessible_description("Writes the file to disk"));
            ui.add(Checkbox::new(&mut on, "Decoration").accessibility_hidden());
            ui.add(Text::new("Status").accessible_role(AccessRole::Heading));
            ui.accessible_group("tools", AccessRole::Toolbar, "Tools", |ui| {
                ui.button("Cut");
                ui.button("Copy");
            });
        });
    });
    let save = harness.node(Role::Button, "Save document");
    assert_eq!(save.description(), Some("Writes the file to disk"));
    assert!(harness.tree.all(Role::CheckBox).is_empty());
    assert!(harness.tree.find(Role::Heading, "Status").is_some());
    let tools = harness.tree.expect(Role::Toolbar, "Tools");
    let names: Vec<_> = harness
        .tree
        .node(tools)
        .children()
        .iter()
        .map(|id| harness.tree.name(*id))
        .collect();
    assert_eq!(names, ["Cut", "Copy"]);
    assert!(
        harness.context.diagnostics().is_empty(),
        "{:?}",
        harness.context.diagnostics()
    );
}

#[test]
fn announcements_are_live_regions_and_repeat() {
    let mut harness = Harness::new();
    let build = |ctx: &mut Context| Window::new("Test").show(ctx, |ui| drop(ui.button("Save")));
    harness.pass(build);
    assert!(harness.tree.all(Role::Status).is_empty());
    harness.context.announce("Saved");
    assert_eq!(
        harness.pass(build),
        Some(2),
        "the message and the window that holds it"
    );
    let status = harness.tree.all(Role::Status)[0];
    assert_eq!(
        harness.tree.node(status).live(),
        Some(accesskit::Live::Polite)
    );
    assert_eq!(
        harness.tree.name(status).trim_end_matches('\u{200b}'),
        "Saved"
    );
    let first = harness.tree.name(status);
    assert_eq!(harness.pass(build), None, "said once");
    // The same text again has to change the node, or nothing would be spoken.
    harness.context.announce("Saved");
    assert_eq!(harness.pass(build), Some(1));
    assert_ne!(harness.tree.name(status), first);
    harness.context.announce_assertive("Disk full");
    harness.pass(build);
    let alert = harness.tree.all(Role::Alert)[0];
    assert_eq!(
        harness.tree.node(alert).live(),
        Some(accesskit::Live::Assertive)
    );
    assert_eq!(harness.tree.name(alert), "Disk full");
}

#[test]
fn announcements_without_a_listener_cost_and_keep_nothing() {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(800, 600), 1.0);
    context.run(|ctx| Window::new("Test").show(ctx, |ui| drop(ui.button("Save"))));
    assert!(!context.needs_repaint_at(context.frame_time()));
    context.announce("Saved");
    assert!(
        !context.needs_repaint_at(context.frame_time()),
        "no frame for nobody"
    );
    assert_eq!(context.accessibility_stats().passes, 0);
}

#[test]
fn repeated_ids_get_distinct_nodes_and_a_diagnostic() {
    let mut harness = Harness::new();
    harness.pass(|ctx| {
        Window::new("Test").show(ctx, |ui| {
            ui.button("Same");
            ui.button("Same");
        });
    });
    assert_eq!(harness.tree.all(Role::Button).len(), 2);
    assert!(harness
        .context
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::IdCollision));
}

#[test]
fn moving_bounds_are_held_while_the_ui_is_in_motion() {
    let mut harness = Harness::new();
    let start = harness.context.frame_time();
    let build = |ctx: &mut Context, x: f32| {
        Window::new("Test").show(ctx, |ui| {
            ui.add_space(x);
            ui.button("Mover");
        });
    };
    harness.context.run_at(start, |ctx| build(ctx, 0.0));
    harness.tree.sync(&mut harness.context);
    let before = harness.node(Role::Button, "Mover").bounds().unwrap();
    // Frames of a motion: something keeps asking for the next frame.
    for frame in 1..=5u32 {
        harness.context.run_at(
            start + Duration::from_millis(16 * u64::from(frame)),
            |ctx| {
                ctx.request_repaint();
                build(ctx, frame as f32 * 10.0);
            },
        );
        assert!(
            harness.tree.sync(&mut harness.context).is_none(),
            "frame {frame} is held"
        );
    }
    // The motion stops: the final bounds are published once.
    harness
        .context
        .run_at(start + Duration::from_millis(96), |ctx| build(ctx, 50.0));
    assert_eq!(harness.tree.sync(&mut harness.context), Some(1));
    let after = harness.node(Role::Button, "Mover").bounds().unwrap();
    assert_eq!(after.y0 - before.y0, 50.0);
    // A motion that never stops still publishes, at a bounded rate.
    let mut published = 0;
    for frame in 0..60u32 {
        harness.context.run_at(
            start + Duration::from_millis(100 + 16 * u64::from(frame)),
            |ctx| {
                ctx.request_repaint();
                build(ctx, 50.0 + frame as f32);
            },
        );
        published += u32::from(harness.tree.sync(&mut harness.context).is_some());
    }
    assert!(
        (2..=8).contains(&published),
        "{published} updates in a second of motion"
    );
}

#[test]
fn semantic_changes_are_never_held() {
    let mut harness = Harness::new();
    let mut on = false;
    let build = |ctx: &mut Context, on: &mut bool| {
        ctx.request_repaint();
        Window::new("Test").show(ctx, |ui| drop(ui.checkbox(on, "Live")));
    };
    harness.pass(|ctx| build(ctx, &mut on));
    on = true;
    assert_eq!(harness.pass(|ctx| build(ctx, &mut on)), Some(1));
}

#[test]
fn a_click_on_a_control_scrolled_out_of_view_reveals_it_and_clicks_once() {
    let mut harness = Harness::new();
    let (mut on, mut clicks) = (false, 0);
    let mut build = |ctx: &mut Context| {
        Root::new().show(ctx, |ui| {
            ScrollArea::vertical().id_source("page").show(ui, |ui| {
                for row in 0..60 {
                    ui.label(format!("Row {row}"));
                }
                clicks += u32::from(ui.checkbox(&mut on, "Far below").clicked());
            });
        });
    };
    harness.settle(&mut build);
    let far = harness.tree.expect(Role::CheckBox, "Far below");
    assert!(harness.tree.node(far).supports_action(Action::Click));
    assert!(harness.act(far, Action::Click));
    harness.settle(&mut build);
    drop(build);
    assert!(on && clicks == 1, "on={on} clicks={clicks}");
    assert_eq!(
        harness.node(Role::CheckBox, "Far below").toggled(),
        Some(Toggled::True)
    );
}
