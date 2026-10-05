use zaxis::{vec2, App, ComboBox, ComboBoxOption, Context, Frame, ScrollArea, Window};

struct Demo {
    selected: Option<usize>,
    small: Option<usize>,
    options: Vec<ComboBoxOption<usize>>,
    changes: usize,
    open: bool,
    update_at: Option<zaxis::Instant>,
    smoke: bool,
    started: Option<zaxis::Instant>,
}
impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        let started = *self.started.get_or_insert(context.frame_time());
        if self
            .update_at
            .is_some_and(|time| context.frame_time() >= time)
        {
            self.options.reverse();
            self.options.retain(|option| option.value % 3 != 0);
            self.update_at = None;
        }
        Window::new("ComboBox")
            .default_position(vec2(30.0, 30.0))
            .default_size(vec2(380.0, 410.0))
            .show(context, |ui| {
                ui.muted("Arrow keys, Enter, Escape; type to filter.");
                let response = ui.add(
                    ComboBox::new(&mut self.selected, &self.options)
                        .id_source("long")
                        .label("region")
                        .filterable(true)
                        .default_open(self.open)
                        .width(300.0),
                );
                self.changes += usize::from(response.changed());
                ui.muted(format!(
                    "Value: {:?}   Changes: {}",
                    self.selected, self.changes
                ));
                if ui.button("Update options in 2 seconds").clicked() {
                    self.update_at =
                        Some(ui.context().frame_time() + std::time::Duration::from_secs(2));
                    ui.context()
                        .request_repaint_after(std::time::Duration::from_secs(2));
                }
                if ui.button("Restore 100 options").clicked() {
                    self.options = options();
                }
                ui.add(
                    ComboBox::new(&mut self.small, &self.options[..self.options.len().min(4)])
                        .id_source("disabled")
                        .label("disabled")
                        .enabled(false),
                );
                ui.muted("The next popup escapes a short ScrollArea.");
                ScrollArea::vertical()
                    .id_source("parent")
                    .max_height(36.0)
                    .show(ui, |ui| {
                        ui.add(
                            ComboBox::new(&mut self.small, &self.options)
                                .id_source("clipped")
                                .label("inside clip")
                                .visible_rows(3),
                        );
                    });
            });
        Window::new("Overlapping window")
            .default_position(vec2(320.0, 170.0))
            .default_size(vec2(280.0, 170.0))
            .show(context, |ui| {
                ui.muted("The popup stays above this window.");
                if ui.button("Count clicks").clicked() {
                    self.changes += 1;
                }
            });
        Window::new("Viewport edge — drag me")
            .default_position(vec2(540.0, 430.0))
            .default_size(vec2(300.0, 145.0))
            .show(context, |ui| {
                ui.add(
                    ComboBox::new(&mut self.small, &self.options)
                        .id_source("edge")
                        .label("opens upward"),
                );
            });
        if self.smoke {
            // Exercise real popup geometry and ScrollArea on the GPU after reveal.
            if context.frame_time().duration_since(started) >= std::time::Duration::from_millis(220)
            {
                frame.close();
            }
        }
    }
}
fn options() -> Vec<ComboBoxOption<usize>> {
    (0..100)
        .map(|i| {
            ComboBoxOption::new(
                i,
                i,
                if i < 2 {
                    "Repeated label".into()
                } else {
                    format!("Region {i:02} / Регион {i:02}")
                },
            )
            .enabled(i != 3 && i != 81)
        })
        .collect()
}
fn main() -> Result<(), zaxis::RunError> {
    let smoke = std::env::args().any(|a| a == "--smoke-test");
    let open = smoke || std::env::args().any(|a| a == "--open");
    zaxis::run_with_options(
        Demo {
            selected: Some(80),
            small: Some(1),
            options: options(),
            changes: 0,
            open,
            update_at: None,
            smoke,
            started: None,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — ComboBox")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(850.0, 620.0)),
            ..Default::default()
        },
    )
}
