use super::{children, nameless, set};
use crate::support::*;

fn switch(ctx: &mut Context, on: &mut bool, changes: &mut u32, enabled: bool) {
    Window::new("Test").show(ctx, |ui| {
        if ui.add(Switch::new(on, "Wi-Fi").enabled(enabled)).changed() {
            *changes += 1;
        }
    });
}

#[test]
fn a_switch_is_toggled_by_one_click_request() {
    let mut harness = Harness::new();
    let (mut on, mut changes) = (false, 0);
    harness.pass(|ctx| switch(ctx, &mut on, &mut changes, true));
    let id = harness.tree.expect(Role::Switch, "Wi-Fi");
    assert_eq!(harness.tree.node(id).toggled(), Some(Toggled::False));
    assert_eq!(
        harness.pass(|ctx| switch(ctx, &mut on, &mut changes, true)),
        None
    );

    assert!(harness.act(id, Action::Click));
    harness.settle(|ctx| switch(ctx, &mut on, &mut changes, true));
    assert!(on);
    assert_eq!(changes, 1, "one request is one change");
    assert_eq!(harness.tree.node(id).toggled(), Some(Toggled::True));
    assert_eq!(
        harness.pass(|ctx| switch(ctx, &mut on, &mut changes, true)),
        None
    );

    harness.settle(|ctx| switch(ctx, &mut on, &mut changes, false));
    assert!(harness.tree.node(id).is_disabled());
    assert!(!harness.act(id, Action::Click));
    harness.settle(|ctx| switch(ctx, &mut on, &mut changes, false));
    assert_eq!((on, changes), (true, 1));
}

#[test]
fn a_switch_without_text_needs_an_accessible_label() {
    let mut harness = Harness::new();
    let mut on = false;
    let mut build = |ctx: &mut Context, named: bool| {
        Window::new("Test").show(ctx, |ui| {
            if named {
                ui.add(Switch::new(&mut on, "").accessible_label("Airplane mode"));
            } else {
                ui.add(Switch::new(&mut on, ""));
            }
        });
    };
    harness.pass(|ctx| build(ctx, false));
    assert_eq!(nameless(&harness), 1);
    harness.pass(|ctx| build(ctx, true));
    assert_eq!(nameless(&harness), 0);
    assert!(harness.tree.find(Role::Switch, "Airplane mode").is_some());
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Level {
    Low,
    Medium,
    High,
    Off,
}

fn radios(ctx: &mut Context, level: &mut Level, changes: &mut u32) {
    Window::new("Test").show(ctx, |ui| {
        let group = RadioGroup::new(
            level,
            [
                RadioOption::new(Level::Low, "Low").description("Quiet fans"),
                RadioOption::new(Level::Medium, "Medium"),
                RadioOption::new(Level::High, "High"),
                RadioOption::new(Level::Off, "Off").enabled(false),
            ],
        );
        if ui.add(group).changed() {
            *changes += 1;
        }
        ui.button("After");
    });
}

#[test]
fn a_radio_group_publishes_every_option_and_selects_on_click() {
    let mut harness = Harness::new();
    let (mut level, mut changes) = (Level::Medium, 0);
    harness.pass(|ctx| radios(ctx, &mut level, &mut changes));
    let group = harness.tree.all(Role::RadioGroup)[0];
    assert_eq!(
        children(&harness, group),
        ["Low", "Medium", "High", "Off"].map(|name| (Role::RadioButton, name.to_owned()))
    );
    let [low, medium, high, off] =
        ["Low", "Medium", "High", "Off"].map(|name| harness.tree.expect(Role::RadioButton, name));
    let toggled = |harness: &Harness| {
        [low, medium, high, off].map(|id| harness.tree.node(id).toggled() == Some(Toggled::True))
    };
    assert_eq!(toggled(&harness), [false, true, false, false]);
    for id in [low, medium, high, off] {
        assert!(
            harness.tree.node(id).toggled().is_some(),
            "every option has a state"
        );
    }
    assert_eq!(set(&harness, low), (Some(0), Some(4)));
    assert_eq!(set(&harness, off), (Some(3), Some(4)));
    assert_eq!(harness.tree.node(low).description(), Some("Quiet fans"));
    assert!(harness.tree.node(off).is_disabled());
    assert!(!harness.tree.node(high).is_disabled());
    assert_eq!(
        harness.pass(|ctx| radios(ctx, &mut level, &mut changes)),
        None
    );

    assert!(harness.act(high, Action::Click));
    harness.settle(|ctx| radios(ctx, &mut level, &mut changes));
    assert_eq!((level, changes), (Level::High, 1));
    assert_eq!(toggled(&harness), [false, false, true, false]);
    // Selecting what is selected changes nothing; a disabled option refuses.
    assert!(harness.act(high, Action::Click));
    assert!(!harness.act(off, Action::Click));
    harness.settle(|ctx| radios(ctx, &mut level, &mut changes));
    assert_eq!((level, changes), (Level::High, 1));
    assert_eq!(
        harness.pass(|ctx| radios(ctx, &mut level, &mut changes)),
        None
    );
}

#[test]
fn focus_in_a_radio_group_is_the_option_holding_the_tab_stop() {
    let mut harness = Harness::new();
    let (mut level, mut changes) = (Level::Medium, 0);
    harness.pass(|ctx| radios(ctx, &mut level, &mut changes));
    let [low, medium] = ["Low", "Medium"].map(|name| harness.tree.expect(Role::RadioButton, name));
    harness
        .context
        .key(KeyCode::Tab, ElementState::Pressed, false);
    harness
        .context
        .key(KeyCode::Tab, ElementState::Released, false);
    harness.settle(|ctx| radios(ctx, &mut level, &mut changes));
    assert_eq!(
        harness.tree.focus(),
        medium,
        "Tab lands on the selected option"
    );
    harness
        .context
        .key(KeyCode::ArrowUp, ElementState::Pressed, false);
    harness
        .context
        .key(KeyCode::ArrowUp, ElementState::Released, false);
    harness.settle(|ctx| radios(ctx, &mut level, &mut changes));
    assert_eq!(harness.tree.focus(), low);
    assert_eq!((level, changes), (Level::Low, 1));
    // The option that holds the Tab stop takes a focus request; the group has one stop.
    harness.context.set_focus(None);
    harness.settle(|ctx| radios(ctx, &mut level, &mut changes));
    assert!(harness.act(low, Action::Focus));
    harness.settle(|ctx| radios(ctx, &mut level, &mut changes));
    assert_eq!(harness.tree.focus(), low);
}

#[test]
fn a_single_radio_value_is_a_radio_button_without_a_group() {
    let mut harness = Harness::new();
    let mut level = Level::Low;
    let mut build = |ctx: &mut Context| {
        Window::new("Test").show(ctx, |ui| {
            ui.radio_value(&mut level, Level::Low, "Low");
            ui.radio_value(&mut level, Level::High, "High");
        });
    };
    harness.pass(&mut build);
    assert!(harness.tree.all(Role::RadioGroup).is_empty());
    let high = harness.tree.expect(Role::RadioButton, "High");
    assert_eq!(set(&harness, high), (None, None));
    assert!(harness.act(high, Action::Click));
    harness.settle(&mut build);
    assert_eq!(harness.tree.node(high).toggled(), Some(Toggled::True));
    let low = harness.tree.expect(Role::RadioButton, "Low");
    assert_eq!(harness.tree.node(low).toggled(), Some(Toggled::False));
}

fn segments(ctx: &mut Context, level: &mut Level, changes: &mut u32, enabled: bool) {
    Window::new("Test").show(ctx, |ui| {
        let control = SegmentedControl::new(
            level,
            [
                SegmentOption::new(Level::Low, "Low"),
                SegmentOption::new(Level::Medium, "Medium"),
                SegmentOption::new(Level::High, "").tooltip("Maximum"),
                SegmentOption::new(Level::Off, "Off").enabled(false),
            ],
        )
        .enabled(enabled);
        if ui.add(control).changed() {
            *changes += 1;
        }
    });
}

#[test]
fn a_segmented_control_is_a_radio_group() {
    let mut harness = Harness::new();
    let (mut level, mut changes) = (Level::Low, 0);
    harness.pass(|ctx| segments(ctx, &mut level, &mut changes, true));
    let group = harness.tree.all(Role::RadioGroup)[0];
    assert_eq!(
        children(&harness, group),
        ["Low", "Medium", "Maximum", "Off"].map(|name| (Role::RadioButton, name.to_owned())),
        "a segment without a label is named by its tooltip"
    );
    assert_eq!(nameless(&harness), 0);
    let [low, medium, off] =
        ["Low", "Medium", "Off"].map(|name| harness.tree.expect(Role::RadioButton, name));
    assert_eq!(harness.tree.node(low).toggled(), Some(Toggled::True));
    assert_eq!(harness.tree.node(medium).toggled(), Some(Toggled::False));
    assert_eq!(set(&harness, medium), (Some(1), Some(4)));
    assert!(harness.tree.node(off).is_disabled());
    assert_eq!(
        harness.pass(|ctx| segments(ctx, &mut level, &mut changes, true)),
        None
    );

    assert!(harness.act(medium, Action::Click));
    harness.settle(|ctx| segments(ctx, &mut level, &mut changes, true));
    assert_eq!((level, changes), (Level::Medium, 1));
    assert_eq!(harness.tree.node(low).toggled(), Some(Toggled::False));
    assert_eq!(harness.tree.node(medium).toggled(), Some(Toggled::True));
    assert!(!harness.act(off, Action::Click));
    assert_eq!(
        harness.pass(|ctx| segments(ctx, &mut level, &mut changes, true)),
        None
    );

    // The keyboard moves focus and selection together; the tree follows both.
    harness
        .context
        .key(KeyCode::Tab, ElementState::Pressed, false);
    harness
        .context
        .key(KeyCode::Tab, ElementState::Released, false);
    harness.settle(|ctx| segments(ctx, &mut level, &mut changes, true));
    assert_eq!(harness.tree.focus(), medium);
    harness
        .context
        .key(KeyCode::ArrowLeft, ElementState::Pressed, false);
    harness
        .context
        .key(KeyCode::ArrowLeft, ElementState::Released, false);
    harness.settle(|ctx| segments(ctx, &mut level, &mut changes, true));
    assert_eq!(harness.tree.focus(), low);
    assert_eq!((level, changes), (Level::Low, 2));

    harness.settle(|ctx| segments(ctx, &mut level, &mut changes, false));
    assert!(harness.tree.node(medium).is_disabled());
    assert!(!harness.act(medium, Action::Click));
    harness.settle(|ctx| segments(ctx, &mut level, &mut changes, false));
    assert_eq!((level, changes), (Level::Low, 2));
}
