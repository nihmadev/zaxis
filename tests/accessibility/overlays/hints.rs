use super::*;
use std::time::Duration;

/// A harness whose tooltips appear without a delay.
fn prompt() -> Harness {
    let mut harness = still();
    let mut style = harness.context.style().clone();
    style.tooltip.delay = Duration::ZERO;
    harness.context.set_style(style);
    harness
}

#[derive(Default)]
struct Form {
    saves: u32,
    volume: f32,
    /// Attach with `Ui::tooltip` after the widget instead of wrapping it.
    direct: bool,
    off: bool,
}

impl Form {
    fn build(&mut self, context: &mut Context) {
        Window::new("Form").show(context, |ui| {
            let save = if self.direct {
                let save = ui.button("Save");
                ui.tooltip(save, "Write the file");
                save
            } else {
                ui.add(
                    Tooltip::new("Write the file")
                        .enabled(!self.off)
                        .wrap(Button::new("Save")),
                )
            };
            self.saves += u32::from(save.clicked());
            ui.add(
                Button::new("Reload")
                    .accessible_description("From disk")
                    .tooltip("Read again"),
            );
            ui.add(Button::new("Help").tooltip("Help"));
            let volume = Slider::new(&mut self.volume, 0.0..=10.0)
                .step(1.0)
                .text("Volume");
            ui.add(volume.tooltip("Output level"));
            ui.add(Text::new("Draft").tooltip("Not saved yet"));
        });
    }
}

#[test]
fn a_tooltip_is_the_description_of_its_widget_shown_or_not() {
    for direct in [false, true] {
        let mut harness = prompt();
        let mut form = Form {
            direct,
            ..Default::default()
        };
        harness.pass(|ctx| form.build(ctx));
        let save = harness.node(Role::Button, "Save");
        assert_eq!(
            save.description(),
            Some("Write the file"),
            "direct: {direct}"
        );
        assert!(save.supports_action(Action::ShowTooltip));
        assert!(save.supports_action(Action::HideTooltip));
        assert!(save.supports_action(Action::Click));
        assert_eq!(
            harness.node(Role::Button, "Reload").description(),
            Some("From disk"),
            "a description the widget already has wins"
        );
        assert_eq!(
            harness.node(Role::Button, "Help").description(),
            None,
            "a tooltip that repeats the name adds nothing"
        );
        assert_eq!(
            harness.node(Role::Slider, "Volume").description(),
            Some("Output level")
        );
        assert_eq!(
            harness.node(Role::Label, "Draft").description(),
            Some("Not saved yet")
        );
        assert!(harness.tree.all(Role::Tooltip).is_empty());
        assert_eq!(harness.pass(|ctx| form.build(ctx)), None);
        assert!(harness.context.diagnostics().is_empty());
    }
}

#[test]
fn a_tooltip_switched_off_says_nothing_and_one_the_theme_hides_still_describes() {
    let mut harness = prompt();
    let mut form = Form {
        off: true,
        ..Default::default()
    };
    harness.pass(|ctx| form.build(ctx));
    let save = harness.node(Role::Button, "Save");
    assert_eq!(save.description(), None);
    assert!(!save.supports_action(Action::ShowTooltip));

    let mut harness = prompt();
    let mut style = harness.context.style().clone();
    style.tooltip.enabled = false;
    harness.context.set_style(style);
    // Attached without an explicit switch, so the theme decides.
    let mut form = Form {
        direct: true,
        ..Default::default()
    };
    harness.pass(|ctx| form.build(ctx));
    let save = harness.tree.expect(Role::Button, "Save");
    assert_eq!(
        harness.tree.node(save).description(),
        Some("Write the file")
    );
    assert!(!harness.tree.node(save).supports_action(Action::ShowTooltip));
    assert!(!harness.act(save, Action::ShowTooltip));
    harness.settle(|ctx| form.build(ctx));
    assert!(harness.tree.all(Role::Tooltip).is_empty());
}

#[test]
fn a_hovered_tooltip_is_a_layer_while_it_shows() {
    let mut harness = prompt();
    let mut form = Form::default();
    harness.pass(|ctx| form.build(ctx));
    let at = middle(harness.node(Role::Button, "Save"));
    harness.context.move_pointer(at);
    harness.settle(|ctx| form.build(ctx));
    let tree = &harness.tree;
    let tip = tree.expect(Role::Tooltip, "Write the file");
    assert_eq!(tree.parent(tip), Some(tree.root()));
    assert_eq!(
        tree.node(tree.root()).children().last(),
        Some(&tip),
        "above everything"
    );
    assert_eq!(harness.pass(|ctx| form.build(ctx)), None);
    assert_eq!(harness.pass(|ctx| form.build(ctx)), None);
    assert!(harness.context.diagnostics().is_empty());
    harness.context.move_pointer(vec2(700.0, 500.0));
    harness.pass(|ctx| form.build(ctx));
    assert!(
        harness.tree.all(Role::Tooltip).is_empty(),
        "gone with the pass that hid it"
    );
    assert_eq!(
        harness.node(Role::Button, "Save").description(),
        Some("Write the file")
    );
    assert_eq!(form.saves, 0);
}

#[test]
fn a_tooltip_shows_and_hides_on_request() {
    for direct in [false, true] {
        let mut harness = prompt();
        let mut form = Form {
            direct,
            ..Default::default()
        };
        harness.pass(|ctx| form.build(ctx));
        let save = harness.tree.expect(Role::Button, "Save");
        assert!(harness.act(save, Action::HideTooltip));
        harness.settle(|ctx| form.build(ctx));
        assert!(
            harness.tree.all(Role::Tooltip).is_empty(),
            "nothing to hide"
        );
        assert!(harness.act(save, Action::ShowTooltip));
        harness.pass(|ctx| form.build(ctx));
        assert!(
            harness.tree.find(Role::Tooltip, "Write the file").is_some(),
            "{direct}"
        );
        harness.settle(|ctx| form.build(ctx));
        assert_eq!(
            harness.pass(|ctx| form.build(ctx)),
            None,
            "it stays without repainting"
        );
        assert_eq!(harness.tree.all(Role::Tooltip).len(), 1);
        assert!(harness.act(save, Action::HideTooltip));
        harness.pass(|ctx| form.build(ctx));
        assert!(
            harness.tree.all(Role::Tooltip).is_empty(),
            "gone with the pass that hid it"
        );
        assert_eq!(form.saves, 0);
    }
}

#[test]
fn a_requested_tooltip_yields_to_the_pointer_and_to_a_press() {
    let mut harness = prompt();
    let mut form = Form::default();
    harness.pass(|ctx| form.build(ctx));
    // A widget that takes the requests for its node itself still shows its tooltip.
    let volume = harness.tree.expect(Role::Slider, "Volume");
    assert!(harness.act(volume, Action::ShowTooltip));
    harness.settle(|ctx| form.build(ctx));
    assert_eq!(names(&harness.tree, Role::Tooltip), ["Output level"]);
    assert!(harness.act(volume, Action::Increment));
    harness.settle(|ctx| form.build(ctx));
    assert_eq!(form.volume, 1.0);
    assert_eq!(names(&harness.tree, Role::Tooltip), ["Output level"]);
    // The pointer's tooltip replaces it, and it does not come back.
    let at = middle(harness.node(Role::Button, "Save"));
    harness.context.move_pointer(at);
    harness.settle(|ctx| form.build(ctx));
    assert_eq!(names(&harness.tree, Role::Tooltip), ["Write the file"]);
    harness.context.move_pointer(vec2(700.0, 500.0));
    harness.settle(|ctx| form.build(ctx));
    assert!(harness.tree.all(Role::Tooltip).is_empty());

    harness.act(volume, Action::ShowTooltip);
    harness.settle(|ctx| form.build(ctx));
    assert_eq!(harness.tree.all(Role::Tooltip).len(), 1);
    harness.context.primary_button(ElementState::Pressed);
    harness.pass(|ctx| form.build(ctx));
    assert!(
        harness.tree.all(Role::Tooltip).is_empty(),
        "a press dismisses it"
    );
    harness.context.primary_button(ElementState::Released);
    harness.settle(|ctx| form.build(ctx));
    assert!(harness.tree.all(Role::Tooltip).is_empty());
}

fn window(context: &mut Context) {
    Window::new("Main").show(context, |ui| {
        ui.button("Save");
    });
}

#[test]
fn a_toast_is_one_live_region_announced_once() {
    let mut harness = Harness::new();
    let mut spoken = Spoken::default();
    let start = Instant::now();
    let mut at = |harness: &mut Harness, ms: u64| {
        pass_at(
            harness,
            start + Duration::from_millis(ms),
            &mut spoken,
            window,
        )
    };
    at(&mut harness, 0);
    assert!(harness.tree.all(Role::Status).is_empty());
    let before = harness.tree.len();
    let toast = Toast::new("Configurations").content("Saved aim.cfg");
    harness
        .context
        .toast(toast.duration(Duration::from_secs(2)));
    assert_eq!(
        at(&mut harness, 16),
        Some(2),
        "the toast and the root that lists it"
    );
    assert_eq!(harness.tree.len(), before + 1);
    let status = harness
        .tree
        .expect(Role::Status, "Configurations: Saved aim.cfg");
    assert_eq!(
        harness.tree.node(status).live(),
        Some(accesskit::Live::Polite)
    );
    assert_eq!(harness.tree.parent(status), Some(harness.tree.root()));
    assert_eq!(
        harness.tree.node(harness.tree.root()).children().last(),
        Some(&status)
    );
    let rest = harness.tree.node(status).bounds();
    // While it slides in, shows and fades out nothing about it is published again.
    for ms in (32..2300).step_by(16) {
        assert_eq!(at(&mut harness, ms), None, "at {ms} ms");
    }
    assert_eq!(harness.tree.node(status).bounds(), rest);
    assert_eq!(harness.context.toast_count(), 1);
    // It leaves the tree with the pass that ends it.
    assert_eq!(at(&mut harness, 2400), Some(1), "only the root changes");
    assert!(harness.tree.all(Role::Status).is_empty());
    assert_eq!(harness.context.toast_count(), 0);
    assert_eq!(at(&mut harness, 2416), None);
    assert_eq!(spoken.0, ["Configurations: Saved aim.cfg"]);
}

#[test]
fn toasts_are_listed_oldest_first_and_a_title_alone_is_the_name() {
    let mut harness = Harness::new();
    let mut spoken = Spoken::default();
    let start = Instant::now();
    let mut at = |harness: &mut Harness, ms: u64| {
        pass_at(
            harness,
            start + Duration::from_millis(ms),
            &mut spoken,
            window,
        )
    };
    at(&mut harness, 0);
    harness.context.toast(Toast::new("Copied"));
    at(&mut harness, 16);
    at(&mut harness, 500);
    harness
        .context
        .toast(Toast::new("Export").content("3 files written"));
    at(&mut harness, 516);
    at(&mut harness, 1000);
    assert_eq!(
        names(&harness.tree, Role::Status),
        ["Copied", "Export: 3 files written"]
    );
    let (first, second) = (
        harness.node(Role::Status, "Copied").bounds().unwrap(),
        harness
            .node(Role::Status, "Export: 3 files written")
            .bounds()
            .unwrap(),
    );
    assert!(first.y1 <= second.y0, "the newest is at the bottom");
    assert_eq!(at(&mut harness, 1016), None);
    assert_eq!(spoken.0, ["Copied", "Export: 3 files written"]);
}

#[test]
fn a_toast_and_a_tooltip_stay_in_the_tree_above_a_modal() {
    let mut harness = prompt();
    let mut open = true;
    let mut build = |ctx: &mut Context| {
        Window::new("Main").show(ctx, |ui| {
            ui.button("Behind");
            Modal::new("m")
                .accessible_label("Notice")
                .show(ui, &mut open, |ui| {
                    ui.add(Button::new("Fine").tooltip("Close this"));
                });
        });
    };
    harness.settle(&mut build);
    harness.context.toast(Toast::new("Saved"));
    let fine = harness.tree.expect(Role::Button, "Fine");
    harness.act(fine, Action::ShowTooltip);
    harness.settle(&mut build);
    let tree = &harness.tree;
    assert!(tree.find(Role::Button, "Behind").is_none());
    let layers: Vec<_> = tree
        .node(tree.root())
        .children()
        .iter()
        .map(|id| tree.node(*id).role())
        .collect();
    assert_eq!(layers, [Role::Dialog, Role::Tooltip, Role::Status]);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn the_title_bar_names_the_window_and_its_caption_buttons() {
    #[derive(Default)]
    struct Chrome {
        maximized: bool,
        seen: [u32; 3],
    }
    let build = |ctx: &mut Context, chrome: &mut Chrome| {
        Root::new().show(ctx, |ui| {
            ui.button("Content");
        });
        let response = TitleBar::new("Notes").maximized(chrome.maximized).show(ctx);
        chrome.seen[0] += u32::from(response.minimize);
        chrome.seen[1] += u32::from(response.maximize);
        chrome.seen[2] += u32::from(response.close);
        chrome.maximized ^= response.maximize;
    };
    let mut harness = still();
    let mut chrome = Chrome::default();
    harness.pass(|ctx| build(ctx, &mut chrome));
    let tree = &harness.tree;
    let bar = tree.expect(Role::TitleBar, "Notes");
    assert_eq!(tree.parent(bar), Some(tree.root()));
    let buttons: Vec<_> = tree
        .node(bar)
        .children()
        .iter()
        .map(|id| (tree.node(*id).role(), tree.name(*id)))
        .collect();
    assert_eq!(
        buttons,
        ["Minimize", "Maximize", "Close"].map(|name| (Role::Button, name.to_owned()))
    );
    assert!(tree.find(Role::Button, "Content").is_some());
    assert_eq!(harness.pass(|ctx| build(ctx, &mut chrome)), None);
    assert!(harness.context.diagnostics().is_empty());

    let minimize = harness.tree.expect(Role::Button, "Minimize");
    assert!(harness.act(minimize, Action::Click));
    harness.settle(|ctx| build(ctx, &mut chrome));
    assert_eq!(chrome.seen, [1, 0, 0]);
    let maximize = harness.tree.expect(Role::Button, "Maximize");
    assert!(harness.act(maximize, Action::Click));
    harness.settle(|ctx| build(ctx, &mut chrome));
    assert_eq!(chrome.seen, [1, 1, 0]);
    assert!(harness.tree.find(Role::Button, "Maximize").is_none());
    let restore = harness.tree.expect(Role::Button, "Restore");
    assert_eq!(restore, maximize, "the same control under its new name");
    let close = harness.tree.expect(Role::Button, "Close");
    assert!(harness.act(close, Action::Click));
    harness.settle(|ctx| build(ctx, &mut chrome));
    assert_eq!(chrome.seen, [1, 1, 1]);
    // The keyboard reaches them as it reaches any button.
    assert!(harness.act(restore, Action::Focus));
    harness.pass(|ctx| build(ctx, &mut chrome));
    assert_eq!(harness.tree.focus(), restore);
    key(&mut harness.context, KeyCode::Enter);
    harness.settle(|ctx| build(ctx, &mut chrome));
    assert_eq!(chrome.seen, [1, 2, 1]);
}
