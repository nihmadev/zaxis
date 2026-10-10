//! Widgets written outside the crate: a knob and a swatch, both made of `Ui::interact`,
//! `Ui::keys` and `Ui::paint`, and a panel of buttons, those widgets, a text field and a
//! slider in one `FocusGroup`. The application owns every value. Drag or arrows turn a
//! knob (Shift: fine, Home/End: ends), double click or the menu resets it. In the panel
//! Tab enters and leaves, Left/Right/Home/End move between controls, Up/Down change the
//! swatch. F3 toggles the debug overlay; `--smoke-test` drives the keyboard headlessly.
#![forbid(unsafe_code)]
use zaxis::{
    vec2,
    winit::{
        event::ElementState,
        keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey},
    },
    AccessRole, App, Border, Button, Color, Column, Context, ContextMenuItem, DebugOverlay,
    FocusGroup, Frame, Grid, InputEvent, KeyInput, KeyInterest, Mods, Response, ScrollArea, Sense,
    Shape, Slider, TextEdit, Ui, Vec2, Widget, WidgetState, Window,
};

const SWATCHES: [Color; 4] = [
    Color::rgb(86, 156, 214),
    Color::rgb(106, 176, 76),
    Color::rgb(224, 169, 74),
    Color::rgb(196, 98, 128),
];

struct Knob<'a> {
    value: &'a mut f32,
    name: &'static str,
}

impl Widget for Knob<'_> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let rect = ui.allocate_space(vec2(56.0, 56.0));
        let mut response = ui.interact(rect, self.name, Sense::CLICK | Sense::DRAG | Sense::FOCUS);
        let before = *self.value;
        if response.double_clicked() {
            *self.value = 0.5;
        }
        if response.dragged() {
            *self.value -= response.drag_delta().y * 0.005;
        }
        // The knob owns these keys while it has focus; Actions and the host never see
        // them. Presses and their autorepeat arrive in order, with the modifiers they had.
        ui.claim_keys(&response, KeyInterest::arrows().repeats());
        ui.claim_keys(
            &response,
            KeyInterest::arrows().with_mods(Mods::SHIFT).repeats(),
        );
        ui.claim_keys(&response, KeyInterest::keys(&[KeyCode::Home, KeyCode::End]));
        for key in ui.take_keys(&response) {
            let step = if key.modifiers.shift_key() {
                0.01
            } else {
                0.05
            };
            match key.code {
                KeyCode::ArrowUp | KeyCode::ArrowRight => *self.value += step,
                KeyCode::ArrowDown | KeyCode::ArrowLeft => *self.value -= step,
                KeyCode::Home => *self.value = 0.0,
                _ => *self.value = 1.0,
            }
        }
        *self.value = self.value.clamp(0.0, 1.0);
        if *self.value != before {
            response.mark_changed();
        }
        let fill = match response.state() {
            WidgetState::Disabled => Color::gray(40),
            WidgetState::Pressed => Color::gray(70),
            WidgetState::Hovered => Color::gray(62),
            WidgetState::Idle => Color::gray(52),
        };
        let accent = ui.style().accent;
        let ring = if response.focus_visible {
            accent
        } else {
            Color::gray(90)
        };
        // Identical descriptions reuse their cached geometry; only a turning value, or a
        // state change, tessellates again.
        ui.paint(Shape::Circle {
            center: rect.center(),
            radius: 26.0,
            fill,
            border: Border::new(if response.focus_visible { 2.0 } else { 1.0 }, ring),
        });
        let angle = (self.value.mul_add(270.0, -135.0)).to_radians();
        let direction = Vec2::new(angle.sin(), -angle.cos());
        ui.paint(Shape::Line {
            start: rect.center() + direction * 8.0,
            end: rect.center() + direction * 21.0,
            width: 3.0,
            color: if response.enabled {
                accent
            } else {
                Color::gray(100)
            },
        });
        response
    }
}

/// A colour chip: a click or Enter/Space steps to the next colour, Up and Down step both
/// ways. It joins the panel's focus group by being built inside it, nothing more.
struct Swatch<'a>(&'a mut usize);

impl Widget for Swatch<'_> {
    fn ui(self, ui: &mut Ui<'_>) -> Response {
        let rect = ui.allocate_space(vec2(28.0, 28.0));
        let mut response = ui.interact(rect, "swatch", Sense::CLICK | Sense::FOCUS);
        let before = *self.0;
        let count = SWATCHES.len();
        if response.clicked() {
            *self.0 = (*self.0 + 1) % count;
        }
        for key in ui.keys(
            &response,
            KeyInterest::keys(&[KeyCode::ArrowUp, KeyCode::ArrowDown]),
        ) {
            *self.0 = if key.code == KeyCode::ArrowUp {
                (*self.0 + 1) % count
            } else {
                (*self.0 + count - 1) % count
            };
        }
        if *self.0 != before {
            response.mark_changed();
        }
        let border = if response.focus_visible {
            ui.style().accent
        } else {
            Color::gray(90)
        };
        ui.paint(
            Shape::rect(rect, SWATCHES[*self.0])
                .corner_radius(6.0)
                .border(Border::new(
                    if response.focus_visible { 2.0 } else { 1.0 },
                    border,
                )),
        );
        response
    }
}

struct Demo {
    values: [f32; 6],
    enabled: bool,
    color: usize,
    name: String,
    master: f32,
    overlay: bool,
    seen: Seen,
    smoke: Option<usize>,
}

/// What the headless check needs to find again: the controls' ids.
#[derive(Default)]
struct Seen {
    knobs: Vec<Response>,
    panel: Vec<(&'static str, Response)>,
}

impl Demo {
    fn view(&mut self, context: &mut Context) {
        if context.input().keys_pressed.contains(&KeyCode::F3) {
            self.overlay = !self.overlay;
            context.set_debug_overlay(if self.overlay {
                DebugOverlay::ALL
            } else {
                DebugOverlay::OFF
            });
        }
        self.seen = Seen::default();
        Window::new("Custom widgets")
            .default_position(vec2(30.0, 30.0))
            .default_size(vec2(400.0, 580.0))
            .show(context, |ui| {
                ui.checkbox(&mut self.enabled, "Enabled");
                ui.add_enabled_ui(self.enabled, |ui| self.knobs(ui));
                ui.separator();
                self.panel(ui);
            });
    }

    fn knobs(&mut self, ui: &mut Ui<'_>) {
        let menu = [ContextMenuItem::new("reset", "Reset to 50%")];
        Grid::new("knobs")
            .columns([
                Column::fixed("name", 90.0),
                Column::fixed("knob", 70.0),
                Column::remainder("value"),
            ])
            .show(ui, |grid| {
                for (index, name) in ["Gain", "Mix", "Tone"].into_iter().enumerate() {
                    grid.row(name, |row| {
                        row.cell(|ui| {
                            ui.label(name);
                        });
                        row.cell(|ui| {
                            let knob = ui.add(
                                Knob {
                                    value: &mut self.values[index],
                                    name,
                                }
                                .context_menu(&menu),
                            );
                            if knob.menu_selected().is_some() {
                                self.values[index] = 0.5;
                            }
                            self.seen.knobs.push(knob);
                        });
                        row.cell(|ui| {
                            ui.small(format!("{:.0}%", self.values[index] * 100.0));
                        });
                    });
                }
            });
        ScrollArea::vertical()
            .id_source("list")
            .max_height(120.0)
            .show(ui, |ui| {
                for (index, name) in ["Low", "Mid", "High"].into_iter().enumerate() {
                    ui.horizontal(|ui| {
                        let knob = ui.add(Knob {
                            value: &mut self.values[3 + index],
                            name,
                        });
                        self.seen.knobs.push(knob);
                        ui.label(name);
                    });
                }
            });
    }

    /// The panel is the application's: a few controls in one group, with the model they
    /// edit next to them. The library supplies the Tab stop, the arrows and the roles.
    fn panel(&mut self, ui: &mut Ui<'_>) {
        let Self {
            values,
            color,
            name,
            master,
            seen,
            ..
        } = self;
        FocusGroup::new("panel")
            .label("Panel")
            .role(AccessRole::Toolbar)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let reset = ui.add(Button::new("Reset"));
                    if reset.clicked() {
                        *values = [0.5; 6];
                    }
                    seen.panel.push(("reset", reset));
                    let swatch = ui.add(Swatch(color).accessible_label("Colour"));
                    seen.panel.push(("swatch", swatch));
                    seen.panel
                        .push(("name", ui.add(TextEdit::new(name).id_source("name"))));
                });
                seen.panel.push((
                    "master",
                    ui.add(Slider::new(master, 0.0..=1.0).step(0.05).text("Master")),
                ));
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

fn demo() -> Demo {
    Demo {
        values: [0.5; 6],
        enabled: true,
        color: 0,
        name: "Mix A".into(),
        master: 0.5,
        overlay: false,
        seen: Seen::default(),
        smoke: None,
    }
}

/// Real keyboard input through `Context::on_input`, no window: the arrows turn a knob, the
/// panel moves focus and changes its model, and a text field keeps its own keys.
fn smoke() -> Result<(), String> {
    let mut context = Context::new();
    context.set_viewport(zaxis::winit::dpi::PhysicalSize::new(900, 700), 1.0);
    let mut app = demo();
    let pass = |app: &mut Demo, context: &mut Context| {
        context.run(|context| app.view(context));
    };
    let key = |context: &mut Context, code: KeyCode, text: Option<&str>| {
        let logical = match (code, text) {
            (KeyCode::ArrowLeft, _) => Key::Named(NamedKey::ArrowLeft),
            (KeyCode::ArrowRight, _) => Key::Named(NamedKey::ArrowRight),
            (KeyCode::ArrowUp, _) => Key::Named(NamedKey::ArrowUp),
            (KeyCode::ArrowDown, _) => Key::Named(NamedKey::ArrowDown),
            (KeyCode::Home, _) => Key::Named(NamedKey::Home),
            (KeyCode::End, _) => Key::Named(NamedKey::End),
            (_, Some(text)) => Key::Character(text.into()),
            _ => Key::Named(NamedKey::Tab),
        };
        for state in [ElementState::Pressed, ElementState::Released] {
            context.on_input(InputEvent::Key(KeyInput {
                physical: PhysicalKey::Code(code),
                logical: logical.clone(),
                state,
                repeat: false,
                text: text
                    .filter(|_| state == ElementState::Pressed)
                    .map(Into::into),
            }));
        }
    };
    pass(&mut app, &mut context);
    pass(&mut app, &mut context);
    // A knob: arrows step by 5%, Shift by 1%, Home and End go to the ends.
    context.request_focus(app.seen.knobs[0].id);
    pass(&mut app, &mut context);
    key(&mut context, KeyCode::ArrowUp, None);
    key(&mut context, KeyCode::ArrowUp, None);
    context.on_input(InputEvent::Modifiers(ModifiersState::SHIFT));
    key(&mut context, KeyCode::ArrowDown, None);
    context.on_input(InputEvent::Modifiers(ModifiersState::empty()));
    pass(&mut app, &mut context);
    check(
        (app.values[0] - 0.59).abs() < 1e-4,
        "arrows 0.5 + 2*0.05 - 0.01",
        app.values[0],
    )?;
    key(&mut context, KeyCode::End, None);
    key(&mut context, KeyCode::Home, None);
    key(&mut context, KeyCode::ArrowRight, None);
    pass(&mut app, &mut context);
    check(
        (app.values[0] - 0.05).abs() < 1e-4,
        "Home then one step",
        app.values[0],
    )?;
    check(
        app.values[1] == 0.5,
        "the other knobs did not move",
        app.values[1],
    )?;
    // The panel: one Tab stop, arrows between the controls, each control's own keys.
    let reset = app.seen.panel[0].1.id;
    context.request_focus(reset);
    pass(&mut app, &mut context);
    key(&mut context, KeyCode::ArrowRight, None);
    pass(&mut app, &mut context);
    check(
        app.seen.panel[1].1.has_focus,
        "Right moved focus to the swatch",
        "",
    )?;
    key(&mut context, KeyCode::ArrowUp, None);
    key(&mut context, KeyCode::ArrowUp, None);
    key(&mut context, KeyCode::ArrowDown, None);
    pass(&mut app, &mut context);
    check(app.color == 1, "Up, Up, Down on the swatch", app.color)?;
    key(&mut context, KeyCode::ArrowRight, None);
    pass(&mut app, &mut context);
    check(
        app.seen.panel[2].1.has_focus,
        "Right moved focus to the text field",
        "",
    )?;
    key(&mut context, KeyCode::Home, None);
    key(&mut context, KeyCode::KeyX, Some("x"));
    pass(&mut app, &mut context);
    check(
        app.name == "xMix A",
        "the text field kept Home and typing",
        &app.name,
    )?;
    key(&mut context, KeyCode::Tab, None);
    pass(&mut app, &mut context);
    check(
        app.seen.panel.iter().all(|(_, r)| !r.has_focus),
        "Tab left the whole panel",
        "",
    )?;
    context.request_focus(app.seen.panel[3].1.id);
    pass(&mut app, &mut context);
    key(&mut context, KeyCode::ArrowRight, None);
    pass(&mut app, &mut context);
    check(app.master > 0.5, "the slider kept its arrows", app.master)?;
    context.request_focus(reset);
    pass(&mut app, &mut context);
    key(&mut context, KeyCode::Space, None);
    pass(&mut app, &mut context);
    pass(&mut app, &mut context);
    check(
        app.values.iter().all(|v| *v == 0.5),
        "Reset button set every knob",
        app.values[0],
    )?;
    Ok(())
}

fn check(ok: bool, what: &str, got: impl std::fmt::Debug) -> Result<(), String> {
    ok.then_some(())
        .ok_or_else(|| format!("{what}: got {got:?}"))
}

fn main() -> Result<(), zaxis::RunError> {
    let smoke_test = std::env::args().any(|arg| arg == "--smoke-test");
    if smoke_test {
        match smoke() {
            Ok(()) => println!("custom widgets smoke: keys, groups and model verified"),
            Err(error) => {
                eprintln!("custom widgets smoke failed: {error}");
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
                .with_title("zaxis — Custom widgets")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(440.0, 620.0)),
            ..Default::default()
        },
    )
}
