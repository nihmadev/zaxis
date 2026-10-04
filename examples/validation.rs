//! A validated form. The application checks its own data on every pass and hands
//! the result to a `Field`; the library only draws the state. F3 toggles the debug overlay.
use zaxis::{
    vec2, winit::keyboard::KeyCode, App, Button, Checkbox, Context, DebugOverlay, Field, Frame,
    NumberInput, SemanticStatus, TextEdit, Validation, Window,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Plan {
    #[default]
    Free,
    Team,
    Business,
}

#[derive(Default)]
struct Form {
    name: String,
    email: String,
    age: u32,
    plan: Plan,
    terms: bool,
    /// Fields show errors after they were left once, or after a submit attempt.
    touched: [bool; 3],
    submitted: bool,
    overlay: bool,
    smoke_test: bool,
    frames: usize,
}

fn name_ok(name: &str) -> Result<(), &'static str> {
    if name.trim().is_empty() {
        Err("Enter your name")
    } else {
        Ok(())
    }
}
fn email_ok(email: &str) -> Result<(), &'static str> {
    match email.split_once('@') {
        Some((user, host)) if !user.is_empty() && host.contains('.') => Ok(()),
        _ => Err("Enter an address like name@example.com"),
    }
}
fn age_ok(age: u32) -> Result<(), String> {
    if (18..=99).contains(&age) {
        Ok(())
    } else {
        Err(format!("Age must be between 18 and 99, not {age}"))
    }
}

impl Form {
    fn valid(&self) -> bool {
        name_ok(&self.name).is_ok()
            && email_ok(&self.email).is_ok()
            && age_ok(self.age).is_ok()
            && self.terms
    }
    /// The result is shown once its field was touched or the form was submitted.
    fn shown<T, E: std::fmt::Display>(&self, field: usize, result: &Result<T, E>) -> Validation {
        if self.touched[field] || self.submitted {
            Validation::from(result)
        } else {
            Validation::ok()
        }
    }
}

impl App for Form {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        if context.input().keys_pressed.contains(&KeyCode::F3) {
            self.overlay = !self.overlay;
            context.set_debug_overlay(if self.overlay {
                DebugOverlay::ALL
            } else {
                DebugOverlay::OFF
            });
        }
        Window::new("Sign up")
            .default_position(vec2(30.0, 30.0))
            .default_size(vec2(380.0, 470.0))
            .show(context, |ui| {
                ui.heading("Create an account");
                let name = self.shown(0, &name_ok(&self.name));
                let left = Field::new("Name").validation(name).show(ui, |ui| {
                    ui.add(TextEdit::new(&mut self.name).id_source("name").width(300.0))
                        .lost_focus()
                });
                self.touched[0] |= left;
                let email = self.shown(1, &email_ok(&self.email));
                let left = Field::new("Email")
                    .hint("Receipts are sent here")
                    .validation(email)
                    .show(ui, |ui| {
                        ui.add(
                            TextEdit::new(&mut self.email)
                                .id_source("email")
                                .width(300.0),
                        )
                        .lost_focus()
                    });
                self.touched[1] |= left;
                let age = self.shown(2, &age_ok(self.age));
                let left = Field::new("Age").validation(age).show(ui, |ui| {
                    ui.add(NumberInput::new(&mut self.age).range(0..=150).width(120.0))
                        .lost_focus()
                });
                self.touched[2] |= left;
                Field::new("Plan")
                    .hint("Change it any time")
                    .show(ui, |ui| {
                        ui.combo_box_values(
                            &mut self.plan,
                            [
                                (Plan::Free, "Free"),
                                (Plan::Team, "Team"),
                                (Plan::Business, "Business"),
                            ],
                        );
                    });
                let terms = if self.submitted && !self.terms {
                    Validation::error("Accept the terms to continue")
                } else {
                    Validation::ok()
                };
                Field::new("").validation(terms).show(ui, |ui| {
                    ui.add(Checkbox::new(&mut self.terms, "I accept the terms"));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add(Button::new("Create account").status(if self.valid() {
                            SemanticStatus::Success
                        } else {
                            SemanticStatus::Normal
                        }))
                        .clicked()
                    {
                        self.submitted = true;
                    }
                    if self.submitted && self.valid() {
                        ui.muted("Account created in memory.");
                    }
                });
            });
        if self.smoke_test {
            self.frames += 1;
            self.submitted |= self.frames > 1;
            if self.frames > 3 {
                frame.close();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Form {
            age: 17,
            smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
            ..Default::default()
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Validation")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(460.0, 560.0)),
            ..Default::default()
        },
    )
}
