use super::children;
use crate::support::*;
use std::time::Duration;

fn key_box(ctx: &mut Context, binding: &mut KeyBinding, changes: &mut u32, enabled: bool) {
    Window::new("Test").show(ctx, |ui| {
        let changed = ui
            .add(KeyBox::new(binding, "Jump").enabled(enabled))
            .changed();
        *changes += u32::from(changed);
    });
}

#[test]
fn a_key_box_is_a_button_that_listens_after_a_click() {
    let mut harness = Harness::new();
    let (mut binding, mut changes) = (KeyBinding::Key(KeyCode::Space), 0);
    harness.pass(|ctx| key_box(ctx, &mut binding, &mut changes, true));
    let button = harness.tree.expect(Role::Button, "Jump");
    assert_eq!(harness.tree.node(button).value(), Some("Space"));
    assert_eq!(harness.tree.node(button).toggled(), Some(Toggled::False));
    assert_eq!(
        harness.pass(|ctx| key_box(ctx, &mut binding, &mut changes, true)),
        None
    );

    assert!(harness.act(button, Action::Click));
    harness.settle(|ctx| key_box(ctx, &mut binding, &mut changes, true));
    assert!(
        harness.context.key_capture_active(),
        "the click started the capture"
    );
    assert_eq!(harness.tree.node(button).toggled(), Some(Toggled::True));
    assert_eq!(changes, 0);
    assert_eq!(
        harness.pass(|ctx| key_box(ctx, &mut binding, &mut changes, true)),
        None,
        "the pulse of a listening box is not news"
    );

    harness
        .context
        .key(KeyCode::KeyK, ElementState::Pressed, false);
    harness
        .context
        .key(KeyCode::KeyK, ElementState::Released, false);
    harness.settle(|ctx| key_box(ctx, &mut binding, &mut changes, true));
    assert_eq!((binding, changes), (KeyBinding::Key(KeyCode::KeyK), 1));
    assert_eq!(harness.tree.node(button).value(), Some("K"));
    assert_eq!(harness.tree.node(button).toggled(), Some(Toggled::False));

    harness.settle(|ctx| key_box(ctx, &mut binding, &mut changes, false));
    assert!(harness.tree.node(button).is_disabled());
    assert!(!harness.act(button, Action::Click));
    harness.settle(|ctx| key_box(ctx, &mut binding, &mut changes, false));
    assert!(!harness.context.key_capture_active());
}

struct Accent {
    color: Color,
    changes: u32,
    enabled: bool,
    floating: bool,
}

impl Accent {
    fn new(floating: bool) -> Self {
        Self {
            color: Color::rgba(255, 0, 0, 128),
            changes: 0,
            enabled: true,
            floating,
        }
    }

    fn build(&mut self, ctx: &mut Context) {
        Window::new("Test").show(ctx, |ui| {
            let picker = ColorPicker::new(&mut self.color, "Accent")
                .id_source("accent")
                .enabled(self.enabled)
                .picker_type(if self.floating {
                    ColorPickerType::Floating
                } else {
                    ColorPickerType::Internal
                });
            self.changes += u32::from(ui.add(picker).changed());
        });
    }
}

/// Passes until the editor has finished opening or closing.
fn rest(harness: &mut Harness, accent: &mut Accent) {
    for _ in 0..3 {
        harness.settle(|ctx| accent.build(ctx));
        std::thread::sleep(Duration::from_millis(250));
    }
    harness.settle(|ctx| accent.build(ctx));
}

fn numeric(harness: &Harness, name: &str) -> Option<f64> {
    harness.node(Role::Slider, name).numeric_value()
}

#[test]
fn a_color_picker_is_a_color_well_that_opens_its_editor() {
    let mut harness = Harness::new();
    let mut accent = Accent::new(false);
    harness.pass(|ctx| accent.build(ctx));
    let well = harness.tree.expect(Role::ColorWell, "Accent");
    let node = harness.tree.node(well);
    assert_eq!(node.value(), Some("#FF0000"));
    let color = node.color_value().expect("a color");
    assert_eq!(
        (color.red, color.green, color.blue, color.alpha),
        (255, 0, 0, 128)
    );
    assert_eq!(node.is_expanded(), Some(false));
    assert!(
        harness.tree.all(Role::Slider).is_empty(),
        "a closed editor is not in the tree"
    );
    assert_eq!(harness.pass(|ctx| accent.build(ctx)), None);

    assert!(harness.act(well, Action::Click));
    rest(&mut harness, &mut accent);
    assert_eq!(harness.tree.node(well).is_expanded(), Some(true));
    assert_eq!(accent.changes, 0, "opening changes no color");
    let group = harness.tree.expect(Role::Group, "Accent");
    assert_eq!(
        children(&harness, group),
        [
            (Role::Slider, "Saturation"),
            (Role::Slider, "Brightness"),
            (Role::Slider, "Hue"),
            (Role::TextInput, "Red"),
            (Role::TextInput, "Green"),
            (Role::TextInput, "Blue"),
            (Role::TextInput, "Hex"),
        ]
        .map(|(role, name)| (role, name.to_owned()))
    );
    assert_eq!(numeric(&harness, "Hue"), Some(0.0));
    assert_eq!(
        harness.node(Role::Slider, "Hue").max_numeric_value(),
        Some(360.0)
    );
    assert_eq!(numeric(&harness, "Saturation"), Some(100.0));
    assert_eq!(numeric(&harness, "Brightness"), Some(100.0));
    assert_eq!(harness.node(Role::TextInput, "Red").value(), Some("255"));
    assert_eq!(
        harness.node(Role::TextInput, "Hex").value(),
        Some("#FF0000")
    );
    assert_eq!(harness.pass(|ctx| accent.build(ctx)), None);

    // Expanding what is open changes nothing; collapsing and expanding are explicit.
    harness.act(well, Action::Expand);
    rest(&mut harness, &mut accent);
    assert_eq!(harness.tree.node(well).is_expanded(), Some(true));
    assert!(harness.act(well, Action::Collapse));
    rest(&mut harness, &mut accent);
    assert_eq!(harness.tree.node(well).is_expanded(), Some(false));
    assert!(harness.tree.all(Role::Slider).is_empty());
    assert!(harness.act(well, Action::Expand));
    rest(&mut harness, &mut accent);
    assert_eq!(harness.tree.all(Role::Slider).len(), 3);
    assert_eq!(accent.changes, 0);
}

#[test]
fn the_editor_of_a_color_picker_takes_values_like_its_keys_and_fields() {
    let mut harness = Harness::new();
    let mut accent = Accent::new(false);
    harness.pass(|ctx| accent.build(ctx));
    let well = harness.tree.expect(Role::ColorWell, "Accent");
    harness.act(well, Action::Click);
    rest(&mut harness, &mut accent);
    let [hue, saturation, brightness] =
        ["Hue", "Saturation", "Brightness"].map(|name| harness.tree.expect(Role::Slider, name));

    assert!(harness.act_with(hue, Action::SetValue, ActionData::NumericValue(120.0)));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(
        (accent.color, accent.changes),
        (Color::rgba(0, 255, 0, 128), 1)
    );
    assert_eq!(harness.tree.node(well).value(), Some("#00FF00"));
    assert_eq!(harness.node(Role::TextInput, "Green").value(), Some("255"));
    assert_eq!(numeric(&harness, "Hue"), Some(120.0));
    assert!(harness.act(hue, Action::Increment));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(
        numeric(&harness, "Hue"),
        Some(124.0),
        "one step is a hundredth, as a key"
    );
    assert_eq!(accent.changes, 2);

    assert!(harness.act_with(brightness, Action::SetValue, ActionData::NumericValue(50.0)));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(numeric(&harness, "Brightness"), Some(50.0));
    assert_eq!(
        numeric(&harness, "Saturation"),
        Some(100.0),
        "the other axis stays"
    );
    assert!(harness.act(saturation, Action::Decrement));
    assert!(harness.act(brightness, Action::Increment));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(numeric(&harness, "Saturation"), Some(99.0));
    assert_eq!(numeric(&harness, "Brightness"), Some(51.0));
    for value in [f64::NAN, f64::INFINITY] {
        harness.act_with(hue, Action::SetValue, ActionData::NumericValue(value));
    }
    harness.act_with(
        saturation,
        Action::SetValue,
        ActionData::NumericValue(-40.0),
    );
    harness.act_with(
        brightness,
        Action::SetValue,
        ActionData::NumericValue(900.0),
    );
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(numeric(&harness, "Hue"), Some(124.0));
    assert_eq!(numeric(&harness, "Saturation"), Some(0.0));
    assert_eq!(numeric(&harness, "Brightness"), Some(100.0));
    assert_eq!(accent.color, Color::rgba(255, 255, 255, 128));

    // The fields commit what they are given by their own rules.
    let changes = accent.changes;
    let [red, hex] = ["Red", "Hex"].map(|name| harness.tree.expect(Role::TextInput, name));
    assert!(harness.act_with(hex, Action::SetValue, ActionData::Value("#0000ff".into())));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(
        (accent.color, accent.changes),
        (Color::rgba(0, 0, 255, 128), changes + 1)
    );
    assert_eq!(harness.tree.node(hex).value(), Some("#0000FF"));
    assert!(harness.act_with(red, Action::SetValue, ActionData::Value("300".into())));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(
        accent.color,
        Color::rgba(255, 0, 255, 128),
        "a channel is clamped"
    );
    for (field, text) in [(hex, "#12"), (hex, "nonsense"), (red, "")] {
        harness.act_with(field, Action::SetValue, ActionData::Value(text.into()));
        harness.settle(|ctx| accent.build(ctx));
        assert_eq!(
            (accent.color, accent.changes),
            (Color::rgba(255, 0, 255, 128), changes + 2)
        );
    }
    assert_eq!(harness.pass(|ctx| accent.build(ctx)), None);

    accent.enabled = false;
    rest(&mut harness, &mut accent);
    assert!(harness.tree.node(well).is_disabled());
    for action in [Action::Click, Action::Expand, Action::Collapse] {
        assert!(!harness.act(well, action));
    }
    for id in harness.tree.all(Role::Slider) {
        assert!(!harness.act(id, Action::Increment));
    }
    rest(&mut harness, &mut accent);
    assert_eq!(accent.color, Color::rgba(255, 0, 255, 128));
}

#[test]
fn a_floating_color_picker_opens_a_window_with_a_close_button() {
    let mut harness = Harness::new();
    let mut accent = Accent::new(true);
    harness.pass(|ctx| accent.build(ctx));
    let well = harness.tree.expect(Role::ColorWell, "Accent");
    assert!(harness.tree.find(Role::Window, "Accent").is_none());
    assert!(harness.act(well, Action::Click));
    rest(&mut harness, &mut accent);
    let window = harness.tree.expect(Role::Window, "Accent");
    assert_eq!(harness.tree.node(well).is_expanded(), Some(true));
    let inside = children(&harness, window);
    assert!(
        inside.contains(&(Role::Slider, "Hue".to_owned())),
        "{inside:?}"
    );
    assert!(
        inside.contains(&(Role::TextInput, "Hex".to_owned())),
        "{inside:?}"
    );
    let hue = harness.tree.expect(Role::Slider, "Hue");
    assert!(harness.act_with(hue, Action::SetValue, ActionData::NumericValue(240.0)));
    harness.settle(|ctx| accent.build(ctx));
    assert_eq!(
        (accent.color, accent.changes),
        (Color::rgba(0, 0, 255, 128), 1)
    );
    assert_eq!(harness.pass(|ctx| accent.build(ctx)), None);

    let close = harness.tree.expect(Role::Button, "Close");
    assert_eq!(harness.tree.parent(close), Some(window));
    assert!(harness.act(close, Action::Click));
    rest(&mut harness, &mut accent);
    assert!(harness.tree.find(Role::Window, "Accent").is_none());
    assert_eq!(harness.tree.node(well).is_expanded(), Some(false));
    assert_eq!(accent.changes, 1);
}
