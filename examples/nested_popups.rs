//! Nested popups: an editor popup per row (text field, combo box, key box) with an
//! Advanced panel opened from inside it, an independent Preview popup, an editor opened
//! from a dialog, and an editor at the edge of the viewport. Opening another root
//! replaces the branch; Escape closes one level at a time. `--smoke-test` drives the
//! pointer and keyboard headlessly and checks the model.
#![forbid(unsafe_code)]
use zaxis::{
    vec2,
    winit::{
        dpi::PhysicalSize,
        event::{ElementState, MouseButton},
        keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    },
    App, Context, Frame, InputEvent, KeyInput, Modal, Popup, Rect, ScrollArea, Ui, Window,
};

#[path = "nested_popups/editor.rs"]
mod editor;
use editor::{Item, Seen};

struct Demo {
    items: Vec<Item>,
    defaults: Item,
    /// One flag per row: the editor of that row is open.
    open: Vec<bool>,
    advanced: Vec<bool>,
    preview: bool,
    dialog: bool,
    dialog_edit: bool,
    dialog_advanced: bool,
    edge: bool,
    edge_advanced: bool,
    edit_buttons: Vec<Rect>,
    preview_button: Rect,
    seen: Seen,
    smoke: Option<usize>,
}

fn demo() -> Demo {
    let names = [
        "Attack", "Decay", "Sustain", "Release", "Vibrato", "Tremolo",
    ];
    let keys = [
        KeyCode::KeyA,
        KeyCode::KeyD,
        KeyCode::KeyS,
        KeyCode::KeyR,
        KeyCode::KeyV,
        KeyCode::KeyT,
    ];
    Demo {
        items: names
            .iter()
            .zip(keys)
            .map(|(n, k)| Item::new(n, k))
            .collect(),
        defaults: Item::new("Defaults", KeyCode::KeyN),
        open: vec![false; names.len()],
        advanced: vec![false; names.len()],
        preview: false,
        dialog: false,
        dialog_edit: false,
        dialog_advanced: false,
        edge: false,
        edge_advanced: false,
        edit_buttons: vec![Rect::default(); names.len()],
        preview_button: Rect::default(),
        seen: Seen::default(),
        smoke: None,
    }
}

impl Demo {
    fn rows(&mut self, ui: &mut Ui<'_>) {
        let mut step = None;
        ScrollArea::vertical()
            .id_source("rows")
            .max_height(150.0)
            .show(ui, |ui| {
                for i in 0..self.items.len() {
                    ui.push_id(i, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(&self.items[i].name);
                            let edit = ui.button("Edit");
                            self.edit_buttons[i] = edit.rect;
                            if edit.clicked() {
                                self.open[i] = !self.open[i];
                            }
                            let asked = editor::show(
                                ui,
                                "editor",
                                edit.rect,
                                &mut self.open[i],
                                &mut self.advanced[i],
                                &mut self.items[i],
                                &mut self.seen,
                            );
                            if asked.next {
                                step = Some(i);
                            }
                        });
                    });
                }
            });
        // "Next" opens the editor of the following row: a second root, which replaces
        // this one. The row that was replaced hears of it on its next pass.
        if let Some(i) = step {
            if let Some(flag) = self.open.get_mut(i + 1) {
                *flag = true;
            }
        }
    }

    fn controls(&mut self, ui: &mut Ui<'_>) {
        ui.horizontal(|ui| {
            let preview = ui.button("Preview");
            self.preview_button = preview.rect;
            if preview.clicked() {
                self.preview = !self.preview;
            }
            Popup::new("preview", preview.rect)
                .size(vec2(220.0, 120.0))
                .show(ui, &mut self.preview, |ui| {
                    for item in &self.items {
                        let mode = item.mode.map_or("-", |m| editor::MODES[m]);
                        ui.label(format!("{}  {}  {:.2}", item.name, mode, item.gain));
                    }
                });
            if ui.button("Dialog").clicked() {
                self.dialog = true;
            }
        });
        let (mut open, mut close) = (self.dialog, false);
        Modal::new("dialog").show(ui, &mut open, |ui| {
            ui.heading("Defaults");
            let edit = ui.button("Edit defaults");
            if edit.clicked() {
                self.dialog_edit = !self.dialog_edit;
            }
            editor::show(
                ui,
                "defaults",
                edit.rect,
                &mut self.dialog_edit,
                &mut self.dialog_advanced,
                &mut self.defaults,
                &mut Seen::default(),
            );
            close = ui.button("Close").clicked();
        });
        self.dialog = open && !close;
        if !self.dialog {
            self.dialog_edit = false;
            self.dialog_advanced = false;
        }
    }

    fn view(&mut self, context: &mut Context) {
        Window::new("Parameters")
            .default_position(vec2(24.0, 24.0))
            .default_size(vec2(300.0, 300.0))
            .show(context, |ui| {
                self.rows(ui);
                ui.separator();
                self.controls(ui);
            });
        Window::new("Edge")
            .default_position(vec2(600.0, 360.0))
            .default_size(vec2(160.0, 90.0))
            .show(context, |ui| {
                let edit = ui.button("Edit");
                if edit.clicked() {
                    self.edge = !self.edge;
                }
                editor::show(
                    ui,
                    "edge",
                    edit.rect,
                    &mut self.edge,
                    &mut self.edge_advanced,
                    &mut self.defaults,
                    &mut Seen::default(),
                );
            });
    }
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        self.view(context);
        if let Some(frames) = &mut self.smoke {
            *frames += 1;
            if *frames > 3 {
                frame.close();
            }
        }
    }
}

fn check(ok: bool, what: &str, got: impl std::fmt::Debug) -> Result<(), String> {
    ok.then_some(())
        .ok_or_else(|| format!("{what}: got {got:?}"))
}

fn click(context: &mut Context, app: &mut Demo, at: impl Fn(&Demo) -> Rect) {
    let p = at(app).center();
    context.on_input(InputEvent::PointerMoved {
        x: f64::from(p.x),
        y: f64::from(p.y),
    });
    for state in [ElementState::Pressed, ElementState::Released] {
        context.on_input(InputEvent::Button {
            button: MouseButton::Left,
            state,
        });
    }
    context.run(|context| app.view(context));
    context.run(|context| app.view(context));
}

fn escape(context: &mut Context, app: &mut Demo, repeats: usize) {
    let event = |state, repeat| {
        InputEvent::Key(KeyInput {
            physical: PhysicalKey::Code(KeyCode::Escape),
            logical: Key::Named(NamedKey::Escape),
            state,
            repeat,
            text: None,
        })
    };
    context.on_input(event(ElementState::Pressed, false));
    for _ in 0..repeats {
        context.on_input(event(ElementState::Pressed, true));
    }
    context.on_input(event(ElementState::Released, false));
    context.run(|context| app.view(context));
    context.run(|context| app.view(context));
}

/// Real pointer and key input through `Context::on_input`, no window.
fn smoke() -> Result<(), String> {
    use zaxis::testing::Inspect;
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(780, 520), 1.0);
    let mut style = context.style().clone();
    style.motion.reduced_motion = true;
    context.set_style(style);
    let mut app = demo();
    for _ in 0..3 {
        context.run(|context| app.view(context));
    }
    let depth = |context: &Context| context.probe().popups.len();
    click(&mut context, &mut app, |a| a.edit_buttons[0]);
    check(
        app.open[0] && depth(&context) == 1,
        "the editor opens",
        depth(&context),
    )?;
    click(&mut context, &mut app, |a| a.seen.more);
    check(
        app.advanced[0] && depth(&context) == 2,
        "Advanced opens inside it",
        depth(&context),
    )?;
    click(&mut context, &mut app, |a| a.seen.looped);
    check(
        app.items[0].looped && depth(&context) == 2,
        "Loop changes the model",
        app.items[0].looped,
    )?;
    // Escape closes the panel only; holding the key does not reach the editor.
    escape(&mut context, &mut app, 3);
    check(
        !app.advanced[0] && app.open[0] && depth(&context) == 1,
        "Escape closes one level",
        depth(&context),
    )?;
    // A combo box in the editor: choosing closes its list, not the editor.
    click(&mut context, &mut app, |a| a.seen.mode);
    check(
        depth(&context) == 2,
        "the list opens above the editor",
        depth(&context),
    )?;
    let leaf = context.probe().popups[1].id;
    for _ in 0..3 {
        context.run(|context| app.view(context));
    }
    let row = context
        .probe()
        .previous_hits
        .iter()
        .filter(|h| h.window == leaf && h.action == zaxis::testing::HitAction::Activate)
        .nth(2)
        .map(|h| h.rect.intersect(h.clip))
        .ok_or("an option row")?;
    click(&mut context, &mut app, |_| row);
    check(
        app.items[0].mode == Some(2) && app.open[0] && depth(&context) == 1,
        "a choice keeps the editor",
        app.items[0].mode,
    )?;
    // Next replaces the editor with the next row's.
    click(&mut context, &mut app, |a| a.seen.next);
    context.run(|context| app.view(context));
    check(
        app.open[1] && !app.open[0] && depth(&context) == 1,
        "another root replaces the branch",
        (app.open[0], app.open[1]),
    )?;
    // A press outside closes the branch and is not a click below.
    click(&mut context, &mut app, |a| a.preview_button);
    check(
        depth(&context) == 0 && !app.preview,
        "an outside press only closes",
        depth(&context),
    )?;
    click(&mut context, &mut app, |a| a.preview_button);
    check(
        app.preview && depth(&context) == 1,
        "Preview opens",
        depth(&context),
    )?;
    Ok(())
}

fn main() -> Result<(), zaxis::RunError> {
    let smoke_test = std::env::args().any(|arg| arg == "--smoke-test");
    if smoke_test {
        match smoke() {
            Ok(()) => println!("nested popups smoke: pointer, keys and model verified"),
            Err(error) => {
                eprintln!("nested popups smoke failed: {error}");
                std::process::exit(1);
            }
        }
    }
    let mut app = demo();
    app.smoke = smoke_test.then_some(0);
    zaxis::run_with_options(
        app,
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Nested popups")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(780.0, 520.0)),
            ..Default::default()
        },
    )
}
