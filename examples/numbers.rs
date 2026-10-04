use zaxis::{vec2, App, Context, DragValue, Frame, NumberInput, Slider, Window};

struct Numbers {
    gain: f32,
    offset: f64,
    count: u64,
    enabled: bool,
    changes: usize,
    smoke: bool,
}
impl App for Numbers {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Window::new("Numeric controls")
            .default_position(vec2(28.0, 24.0))
            .default_size(vec2(540.0, 550.0))
            .min_size(vec2(510.0, 540.0))
            .show(context, |ui| {
                ui.title("Numbers");
                ui.checkbox(&mut self.enabled, "Enable editing");
                ui.add_enabled_ui(self.enabled, |ui| {
                    ui.muted("Gain");
                    ui.horizontal_aligned(zaxis::Align::Center, |ui| {
                        self.changes += usize::from(
                            ui.add(
                                Slider::new(&mut self.gain, 0.0..=1.0)
                                    .step(0.01)
                                    .width(290.0),
                            )
                            .changed(),
                        );
                        self.changes += usize::from(
                            ui.add(
                                NumberInput::new(&mut self.gain)
                                    .range(0.0..=1.0)
                                    .step(0.01)
                                    .precision(2)
                                    .suffix(" ×")
                                    .width(100.0),
                            )
                            .changed(),
                        );
                    });
                    ui.muted("Offset · drag horizontally or click to type");
                    self.changes += usize::from(
                        ui.add(
                            DragValue::new(&mut self.offset)
                                .range(-100.0..=100.0)
                                .step(0.1)
                                .sensitivity(0.2)
                                .precision(2)
                                .suffix(" px")
                                .width(160.0),
                        )
                        .changed(),
                    );
                    ui.muted("Exact u64 · beyond the precision of f64");
                    self.changes += usize::from(
                        ui.add(NumberInput::new(&mut self.count).width(260.0))
                            .changed(),
                    );
                });
                ui.separator();
                ui.muted("↑ / ↓ adjust · Shift precise · Ctrl fast");
                ui.muted("Enter confirms · Escape cancels · valid input commits on blur");
                ui.muted(format!("Number changes: {}", self.changes));
            });
        if self.smoke {
            frame.close();
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Numbers {
            gain: 0.65,
            offset: -12.5,
            count: 9_007_199_254_740_993,
            enabled: true,
            changes: 0,
            smoke: std::env::args().any(|v| v == "--smoke-test"),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — NumberInput & DragValue")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(600.0, 610.0)),
            ..Default::default()
        },
    )
}
