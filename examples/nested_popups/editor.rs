//! The parameter editor: a popup with a text field, a combo box, a key box and a child
//! panel. The model is plain data owned by the application.
use zaxis::{
    vec2, winit::keyboard::KeyCode, ComboBox, ComboBoxOption, KeyBinding, KeyBox, Popup, Rect,
    Slider, TextEdit, Ui,
};

pub const MODES: [&str; 4] = ["Linear", "Smooth", "Steps", "Hold"];

pub struct Item {
    pub name: String,
    pub mode: Option<usize>,
    pub key: KeyBinding,
    pub gain: f32,
    pub looped: bool,
}

impl Item {
    pub fn new(name: &str, key: KeyCode) -> Self {
        Self {
            name: name.into(),
            mode: Some(0),
            key: KeyBinding::Key(key),
            gain: 0.5,
            looped: false,
        }
    }
}

/// Where the editor's controls were last drawn, for the headless check.
#[derive(Default, Clone, Copy)]
pub struct Seen {
    pub more: Rect,
    pub looped: Rect,
    pub mode: Rect,
    pub next: Rect,
}

/// What a pass of the editor asks of its caller.
#[derive(Default)]
pub struct Asked {
    pub next: bool,
}

/// The editor of `item` as a popup under `anchor`. Its Advanced panel is a popup opened
/// from inside it; both close from outside (an outside press, Escape) without the caller
/// doing anything but reading the flags back.
pub fn show(
    ui: &mut Ui<'_>,
    source: impl std::hash::Hash,
    anchor: Rect,
    open: &mut bool,
    advanced: &mut bool,
    item: &mut Item,
    seen: &mut Seen,
) -> Asked {
    let mut asked = Asked::default();
    let modes: Vec<_> = MODES
        .iter()
        .enumerate()
        .map(|(i, name)| ComboBoxOption::new(i, i, *name))
        .collect();
    Popup::new(source, anchor)
        .size(vec2(280.0, 190.0))
        .show(ui, open, |ui| {
            ui.add(TextEdit::new(&mut item.name).id_source("name"));
            let mode = ui.add(
                ComboBox::new(&mut item.mode, &modes)
                    .id_source("mode")
                    .label("Mode"),
            );
            seen.mode = mode.rect;
            ui.add(KeyBox::new(&mut item.key, "Key").id_source("key"));
            ui.horizontal(|ui| {
                let more = ui.button("Advanced");
                seen.more = more.rect;
                if more.clicked() {
                    *advanced = !*advanced;
                }
                let next = ui.button("Next");
                seen.next = next.rect;
                asked.next = next.clicked();
                Popup::new("advanced", more.rect)
                    .size(vec2(200.0, 90.0))
                    .show(ui, advanced, |ui| {
                        ui.add(Slider::new(&mut item.gain, 0.0..=1.0).text("Gain"));
                        let looped = ui.checkbox(&mut item.looped, "Loop");
                        seen.looped = looped.rect;
                    });
            });
        });
    if !*open {
        // What the closed editor had open does not come back with it.
        *advanced = false;
    }
    asked
}
