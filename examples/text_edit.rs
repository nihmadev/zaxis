use zaxis::{vec2, App, Context, Frame, ScrollArea, TextEdit, Window};

struct Editor {
    name: String,
    note: String,
    notes: String,
    code: String,
    log: String,
    mixed: String,
    disabled: String,
    lines: usize,
    changes: usize,
    submits: usize,
    smoke_test: bool,
}

impl Editor {
    fn long_text() -> String {
        (1..=60)
            .map(|n| {
                format!("Line {n}: the quick brown fox jumps over the lazy dog, again and again.")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl App for Editor {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Window::new("TextEdit")
            .default_size(vec2(560.0, 620.0))
            .show(context, |ui| {
                ScrollArea::vertical().max_height(f32::MAX).show(ui, |ui| {
                    ui.label("Name / Имя");
                    let response = ui.add(
                        TextEdit::new(&mut self.name)
                            .id_source("name")
                            .placeholder("Введите имя")
                            .width(320.0),
                    );
                    self.changes += usize::from(response.changed());
                    ui.label("Note: grows from 2 to 8 lines, Ctrl+Enter submits");
                    let response = ui.add(
                        TextEdit::new(&mut self.note)
                            .id_source("note")
                            .multiline()
                            .auto_height(2.0, 8.0)
                            .submit_on_ctrl_enter(true)
                            .placeholder("Write a note…"),
                    );
                    if response.submitted() {
                        if !self.note.is_empty() {
                            if !self.notes.is_empty() {
                                self.notes.push('\n');
                            }
                            self.notes.push_str(&self.note.replace('\n', " "));
                        }
                        self.note.clear();
                        self.submits += 1;
                    }
                    ui.label("Submitted notes (read-only)");
                    ui.add(
                        TextEdit::new(&mut self.notes)
                            .id_source("notes")
                            .read_only(true)
                            .rows(3.0),
                    );
                    ui.label("Fixed height with scrolling");
                    ui.add(
                        TextEdit::new(&mut self.log)
                            .id_source("fixed")
                            .multiline()
                            .rows(5.0),
                    );
                    if ui.button("Append line").clicked() {
                        self.lines += 1;
                        self.log
                            .push_str(&format!("\nAppended line {}", self.lines));
                    }
                    ui.label("No wrapping, Tab inserts an indent");
                    ui.add(
                        TextEdit::new(&mut self.code)
                            .id_source("code")
                            .wrap(false)
                            .tab_indent(true)
                            .tab_size(4)
                            .rows(4.0),
                    );
                    ui.label("Cyrillic · emoji · العربية · עברית");
                    ui.add(
                        TextEdit::new(&mut self.mixed)
                            .id_source("mixed")
                            .multiline()
                            .rows(5.0),
                    );
                    ui.label("Disabled");
                    ui.add(
                        TextEdit::new(&mut self.disabled)
                            .id_source("disabled")
                            .multiline()
                            .enabled(false)
                            .rows(2.0),
                    );
                    ui.muted(format!(
                        "Changes: {}   Submitted: {}",
                        self.changes, self.submits
                    ));
                });
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
            note: "Кириллица · е\u{301} · 👩‍💻 · 🇷🇺\nSecond line".into(),
            notes: String::new(),
            code: "fn main() {\n\tprintln!(\"a long line that does not wrap and scrolls horizontally in the field\");\n}".into(),
            log: Editor::long_text(),
            mixed: "Привет, мир! 👩‍💻🇷🇺\nمرحبا بالعالم — النص العربي\nשלום עולם — טקסט בעברית\nmixed: abc مرحبا def שלום".into(),
            disabled: "Unavailable\nmulti-line".into(),
            lines: 0,
            changes: 0,
            submits: 0,
            smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — TextEdit")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(640.0, 700.0)),
            ..Default::default()
        },
    )
}
