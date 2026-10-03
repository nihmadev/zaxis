use zaxis::{vec2, App, Context, Frame, TextEdit, Window};

struct Editor {
    name: String,
    note: String,
    read_only: String,
    disabled: String,
    changes: usize,
    submits: usize,
    blurs: usize,
    smoke_test: bool,
}

impl App for Editor {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Window::new("TextEdit")
            .default_size(vec2(530.0, 450.0))
            .show(context, |ui| {
                ui.label("Name / Имя");
                let response = ui.add(
                    TextEdit::new(&mut self.name)
                        .id_source("name")
                        .placeholder("Введите имя")
                        .width(320.0),
                );
                self.changes += usize::from(response.changed());
                self.submits += usize::from(response.submitted());
                self.blurs += usize::from(response.lost_focus());
                ui.label("Independent field / Отдельное поле");
                ui.text_edit(&mut self.note);
                ui.label("Read-only: select and copy");
                ui.add(
                    TextEdit::new(&mut self.read_only)
                        .read_only(true)
                        .width(320.0),
                );
                ui.label("Disabled");
                ui.add(
                    TextEdit::new(&mut self.disabled)
                        .disabled(true)
                        .width(320.0),
                );
                ui.muted(format!(
                    "Changes: {}   Enter: {}   Blur: {}",
                    self.changes, self.submits, self.blurs
                ));
            });
        if self.smoke_test {
            frame.close();
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Editor {
            name: String::new(),
            note: "Кириллица · е\u{301} · 👩‍💻 · 🇷🇺".into(),
            read_only: "Select this text with Shift or the mouse".into(),
            disabled: "Unavailable".into(),
            changes: 0,
            submits: 0,
            blurs: 0,
            smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — TextEdit")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(620.0, 520.0)),
            ..Default::default()
        },
    )
}
