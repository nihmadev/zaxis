use zaxis::{
    vec2, App, Button, Color, Context, Frame, Gradient, GradientDirection, Hover, HoverStyle,
    PresentationMode, Rect, RunOptions, Separator, Shadow, Shape, Switch, Window,
};

struct Demo {
    clicks: u64,
    checked: bool,
    notifications: bool,
    smoke_test: bool,
    smoke_frames: usize,
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        let targets = show_ui(
            context,
            &mut self.clicks,
            &mut self.checked,
            &mut self.notifications,
        );
        if self.smoke_test {
            if self.smoke_frames >= targets.len() {
                frame.close();
            } else {
                let pointer = targets[self.smoke_frames].center() * context.scale_factor();
                context.on_window_event(&zaxis::winit::event::WindowEvent::CursorMoved {
                    device_id: zaxis::winit::event::DeviceId::dummy(),
                    position: zaxis::winit::dpi::PhysicalPosition::new(
                        f64::from(pointer.x),
                        f64::from(pointer.y),
                    ),
                });
                self.smoke_frames += 1;
                context.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    let options = RunOptions {
        window_attributes: zaxis::winit::window::Window::default_attributes()
            .with_title("zaxis — desktop GUI")
            .with_inner_size(zaxis::winit::dpi::LogicalSize::new(860.0, 560.0)),
        presentation_mode: if std::env::args().any(|arg| arg == "--vsync") {
            PresentationMode::Vsync
        } else {
            PresentationMode::Immediate
        },
        ..Default::default()
    };
    zaxis::run_with_options(
        Demo {
            clicks: 0,
            checked: true,
            notifications: true,
            smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
            smoke_frames: 0,
        },
        options,
    )
}

fn show_ui(
    context: &mut Context,
    clicks: &mut u64,
    checked: &mut bool,
    notifications: &mut bool,
) -> Vec<Rect> {
    let mut targets = Vec::new();
    Window::new("zaxis")
        .default_position(vec2(56.0, 48.0))
        .default_size(vec2(410.0, 410.0))
        .min_size(vec2(270.0, 230.0))
        .show(context, |ui| {
            ui.title("A small desktop UI");
            ui.label("Immediate mode widgets, cached geometry, and an event-driven renderer.");
            ui.separator();
            let response = ui.button("Click me");
            targets.push(response.rect);
            if response.clicked() { *clicks += 1; }
            ui.label(format!("Clicks: {clicks}"));
            ui.checkbox(checked, "Show checkmark");
            ui.switch(notifications, "Notifications");
            ui.add(Switch::new(checked, "Show checkmark when on").enabled(*notifications));
            ui.add(Separator::new().thickness(2.0).inset(8.0).spacing(2.0));
            ui.muted("Drag the title bar or resize the lower-right corner. Tab focuses the button; Space or Enter clicks it.");
        });
    Window::new("Hover styles")
        .default_position(vec2(488.0, 48.0))
        .default_size(vec2(320.0, 480.0))
        .min_size(vec2(280.0, 420.0))
        .show(context, |ui| {
            ui.label("Move the pointer over each control.");
            let width = ui.available_width();
            let button = |label| Button::new(label).min_size(vec2(width, 38.0));
            for (label, hover) in [
                ("Soft shadow", HoverStyle::shadow(Shadow::default())),
                (
                    "Gradient",
                    HoverStyle::gradient(
                        Gradient::new(Color::rgb(58, 91, 145), Color::rgb(99, 65, 137))
                            .direction(GradientDirection::Diagonal),
                    ),
                ),
                ("Subtle fill", HoverStyle::fill(Color::gray(88))),
                (
                    "Glow",
                    HoverStyle::shadow(Shadow {
                        color: Color::rgba(91, 147, 230, 120),
                        offset: vec2(0.0, 0.0),
                        ..Shadow::default()
                    }),
                ),
            ] {
                targets.push(ui.add(button(label).hover_style(hover)).rect);
            }
            let custom = ui.add(
                Hover::new(button("Custom accent"))
                    .style(HoverStyle::NONE)
                    .on_hover(|ui, response| {
                        let rect = Rect::from_min_size(
                            response.rect.min + vec2(10.0, 8.0),
                            vec2(3.0, response.rect.size().y - 16.0),
                        );
                        ui.paint(Shape::rect(rect, Color::rgb(107, 192, 159)).corner_radius(1.5));
                    }),
            );
            targets.push(custom.rect);
            ui.add(button("Disabled").enabled(false));
            ui.add(Separator::new());
            ui.muted(
                "Hover changes appearance. Tab shows keyboard focus; pressing takes priority.",
            );
        });
    targets
}
