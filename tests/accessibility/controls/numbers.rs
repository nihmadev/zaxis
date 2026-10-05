use super::{nameless, platform_names};
use crate::support::*;

struct Gain {
    value: f64,
    changes: u32,
    enabled: bool,
    drag: bool,
}

impl Gain {
    fn new(drag: bool) -> Self {
        Self {
            value: 2.0,
            changes: 0,
            enabled: true,
            drag,
        }
    }

    fn build(&mut self, ctx: &mut Context) {
        Window::new("Test").show(ctx, |ui| {
            let changed = if self.drag {
                let control = DragValue::new(&mut self.value)
                    .id_source("gain")
                    .range(0.0..=10.0)
                    .step(0.5)
                    .suffix(" dB")
                    .enabled(self.enabled);
                ui.add(control.accessible_label("Gain")).changed()
            } else {
                let control = NumberInput::new(&mut self.value)
                    .id_source("gain")
                    .range(0.0..=10.0)
                    .step(0.5)
                    .suffix(" dB")
                    .enabled(self.enabled);
                ui.add(control.accessible_label("Gain")).changed()
            };
            self.changes += u32::from(changed);
            ui.button("Next");
        });
    }
}

fn press(harness: &mut Harness, key: KeyCode) {
    harness.context.key(key, ElementState::Pressed, false);
    harness.context.key(key, ElementState::Released, false);
}

/// The requests a spin button takes, whichever widget it is.
fn value_requests(harness: &mut Harness, gain: &mut Gain) {
    let spin = harness.tree.expect(Role::SpinButton, "Gain");
    let node = harness.tree.node(spin);
    assert_eq!(node.numeric_value(), Some(2.0));
    assert_eq!(
        (node.min_numeric_value(), node.max_numeric_value()),
        (Some(0.0), Some(10.0))
    );
    assert_eq!(node.numeric_value_step(), Some(0.5));
    assert_eq!(
        node.value(),
        Some("2 dB"),
        "the value is the text on screen"
    );
    assert_eq!(nameless(harness), 0);
    assert_eq!(harness.pass(|ctx| gain.build(ctx)), None);

    assert!(harness.act(spin, Action::Increment));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (2.5, 1));
    assert!(harness.act(spin, Action::Decrement));
    assert!(harness.act(spin, Action::Decrement));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(
        (gain.value, gain.changes),
        (1.5, 2),
        "two requests in one pass are one change"
    );
    assert_eq!(harness.tree.node(spin).numeric_value(), Some(1.5));
    assert_eq!(harness.tree.node(spin).value(), Some("1.5 dB"));

    // A value set from outside is clamped like a typed one, and not snapped to the step.
    assert!(harness.act_with(spin, Action::SetValue, ActionData::NumericValue(7.3)));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (7.3, 3));
    harness.act_with(spin, Action::SetValue, ActionData::NumericValue(99.0));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (10.0, 4));
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        harness.act_with(spin, Action::SetValue, ActionData::NumericValue(value));
        harness.settle(|ctx| gain.build(ctx));
        assert_eq!((gain.value, gain.changes), (10.0, 4));
    }
    assert!(harness.act(spin, Action::Increment));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(
        (gain.value, gain.changes),
        (10.0, 4),
        "the upper bound holds"
    );
    for (text, expected) in [("4.5", 4.5), ("3 dB", 3.0), ("-8", 0.0)] {
        assert!(harness.act_with(spin, Action::SetValue, ActionData::Value(text.into())));
        harness.settle(|ctx| gain.build(ctx));
        assert_eq!(gain.value, expected, "{text}");
    }
    let changes = gain.changes;
    for text in ["", "loud", "1e999", "NaN"] {
        harness.act_with(spin, Action::SetValue, ActionData::Value(text.into()));
        harness.settle(|ctx| gain.build(ctx));
        assert_eq!(
            (gain.value, gain.changes),
            (0.0, changes),
            "{text:?} is no number"
        );
    }
    assert_eq!(harness.pass(|ctx| gain.build(ctx)), None);

    gain.enabled = false;
    harness.settle(|ctx| gain.build(ctx));
    assert!(harness.tree.node(spin).is_disabled());
    assert!(!harness.act(spin, Action::Increment));
    assert!(!harness.act_with(spin, Action::SetValue, ActionData::NumericValue(5.0)));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (0.0, changes));
}

#[test]
fn a_number_input_is_a_spin_button_around_its_text_field() {
    let mut harness = Harness::new();
    let mut gain = Gain::new(false);
    harness.pass(|ctx| gain.build(ctx));
    let spin = harness.tree.expect(Role::SpinButton, "Gain");
    let fields = harness.tree.node(spin).children().to_vec();
    assert_eq!(
        fields.len(),
        1,
        "the text field is the spin button's only child"
    );
    let field = fields[0];
    assert_eq!(harness.tree.node(field).role(), Role::TextInput);
    assert_eq!(
        harness.tree.name(field),
        "Gain",
        "the field is named after the spin button"
    );
    assert_eq!(
        harness.tree.all(Role::TextInput),
        [field],
        "and is published once"
    );
    assert_eq!(
        platform_names(&harness, Role::TextInput),
        [Some("Gain".to_owned())]
    );
    value_requests(&mut harness, &mut gain);
}

#[test]
fn a_draft_is_committed_before_a_step_and_the_field_holds_the_focus() {
    let mut harness = Harness::new();
    let mut gain = Gain::new(false);
    harness.pass(|ctx| gain.build(ctx));
    let spin = harness.tree.expect(Role::SpinButton, "Gain");
    let field = harness.tree.node(spin).children()[0];
    assert!(harness.act(field, Action::Focus));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(harness.tree.focus(), field);
    assert_eq!(
        harness.tree.node(field).value(),
        Some("2"),
        "editing shows the plain number"
    );

    harness.context.on_text_event("7");
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(
        (gain.value, gain.changes),
        (2.0, 0),
        "a draft is not a value yet"
    );
    assert_eq!(harness.tree.node(spin).value(), Some("7 dB"));
    assert!(harness.act(spin, Action::Increment));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (7.5, 1));
    assert_eq!(harness.tree.node(field).value(), Some("7.5"));

    // A draft that is no number is kept and marked; the value does not become zero.
    harness.context.on_text_event("x");
    harness.settle(|ctx| gain.build(ctx));
    harness.act(spin, Action::Increment);
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (7.5, 1));
    assert!(harness.tree.node(spin).invalid().is_some());

    // A value set on the text field itself is committed as a number too.
    assert!(harness.act_with(field, Action::SetValue, ActionData::Value("4".into())));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!((gain.value, gain.changes), (4.0, 2));
    assert!(harness.tree.node(spin).invalid().is_none());
    press(&mut harness, KeyCode::ArrowUp);
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(
        (gain.value, gain.changes),
        (4.5, 3),
        "the keys continue from the set value"
    );
}

#[test]
fn a_drag_value_is_a_spin_button_that_holds_the_focus_itself() {
    let mut harness = Harness::new();
    let mut gain = Gain::new(true);
    harness.pass(|ctx| gain.build(ctx));
    let spin = harness.tree.expect(Role::SpinButton, "Gain");
    assert!(harness.tree.node(spin).children().is_empty());
    assert!(harness.act(spin, Action::Focus));
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(harness.tree.focus(), spin);
    // Enter turns it into a text field: the same spin button, now around the field.
    press(&mut harness, KeyCode::Enter);
    harness.settle(|ctx| gain.build(ctx));
    assert_eq!(harness.tree.expect(Role::SpinButton, "Gain"), spin);
    let field = harness.tree.node(spin).children()[0];
    assert_eq!(harness.tree.node(field).role(), Role::TextInput);
    assert_eq!(harness.tree.focus(), field);
    press(&mut harness, KeyCode::Escape);
    harness.settle(|ctx| gain.build(ctx));
    assert!(harness.tree.node(spin).children().is_empty());
    assert_eq!(harness.tree.focus(), spin);
    harness.context.set_focus(None);
    harness.settle(|ctx| gain.build(ctx));
    value_requests(&mut harness, &mut gain);
}

#[test]
fn an_unbounded_number_publishes_its_text_only_and_needs_a_name() {
    let mut harness = Harness::new();
    let mut count = 41_i32;
    let mut build = |ctx: &mut Context| {
        Window::new("Test").show(ctx, |ui| {
            ui.add(NumberInput::new(&mut count).id_source("count"));
        });
    };
    harness.pass(&mut build);
    assert_eq!(
        nameless(&harness),
        1,
        "the spin button, not its text field as well"
    );
    let spin = harness.tree.all(Role::SpinButton)[0];
    let node = harness.tree.node(spin);
    assert_eq!(node.value(), Some("41"));
    assert_eq!(node.numeric_value(), None);
    assert!(harness.act(spin, Action::Increment));
    harness.settle(&mut build);
    assert_eq!(harness.tree.node(spin).value(), Some("42"));
    harness.act_with(spin, Action::SetValue, ActionData::NumericValue(1e300));
    harness.settle(&mut build);
    assert_eq!(
        harness.tree.node(spin).value(),
        Some(i32::MAX.to_string().as_str())
    );
}
