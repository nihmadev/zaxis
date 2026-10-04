//! A widget written outside the library: allocate, `Ui::interact`, paint.
//! The knob works in a Grid, a ScrollArea and a disabled group like any built-in
//! control. Drag to turn, arrows step, double click resets, right click opens a
//! menu. F3 toggles the debug overlay.
use zaxis::{
    vec2, winit::keyboard::KeyCode, App, Border, Color, Column, Context, ContextMenuItem,
    DebugOverlay, Frame, Grid, Response, ScrollArea, Sense, Shape, Ui, Vec2, Widget, Window,
};

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
        if response.has_focus {
            let keys = &ui.context().input().keys_pressed;
            let step = f32::from(
                i8::from(keys.contains(&KeyCode::ArrowUp))
                    - i8::from(keys.contains(&KeyCode::ArrowDown)),
            );
            *self.value += step * 0.05;
        }
        *self.value = self.value.clamp(0.0, 1.0);
        if *self.value != before {
            response.mark_changed();
        }
        let fill = match response.state() {
            zaxis::WidgetState::Disabled => Color::gray(40),
            zaxis::WidgetState::Pressed => Color::gray(70),
            zaxis::WidgetState::Hovered => Color::gray(62),
            zaxis::WidgetState::Idle => Color::gray(52),
        };
        let accent = ui.style().accent;
        // Identical descriptions reuse their cached geometry; only a turning
        // value, or a state change, tessellates again.
        ui.paint(Shape::Circle {
            center: rect.center(),
            radius: 26.0,
            fill,
            border: Border::new(
                if response.focus_visible { 2.0 } else { 1.0 },
                if response.focus_visible {
                    accent
                } else {
                    Color::gray(90)
                },
            ),
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

struct Demo {
    values: [f32; 6],
    enabled: bool,
    overlay: bool,
    smoke_test: bool,
    frames: usize,
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        if context.input().keys_pressed.contains(&KeyCode::F3) {
            self.overlay = !self.overlay;
            context.set_debug_overlay(if self.overlay {
                DebugOverlay::ALL
            } else {
                DebugOverlay::OFF
            });
        }
        let menu = [ContextMenuItem::new("reset", "Reset to 50%")];
        Window::new("Custom widget")
            .default_position(vec2(30.0, 30.0))
            .default_size(vec2(360.0, 420.0))
            .show(context, |ui| {
                ui.checkbox(&mut self.enabled, "Enabled");
                ui.add_enabled_ui(self.enabled, |ui| {
                    Grid::new("knobs")
                        .columns([
                            Column::fixed("name", 110.0),
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
                                    });
                                    row.cell(|ui| {
                                        ui.small(format!("{:.0}%", self.values[index] * 100.0));
                                    });
                                });
                            }
                        });
                    ui.separator();
                    ScrollArea::vertical()
                        .id_source("list")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            for (index, name) in ["Low", "Mid", "High"].into_iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add(Knob {
                                        value: &mut self.values[3 + index],
                                        name,
                                    });
                                    ui.label(name);
                                });
                            }
                        });
                });
            });
        if self.smoke_test {
            self.frames += 1;
            if self.frames > 3 {
                frame.close();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            values: [0.5; 6],
            enabled: true,
            overlay: false,
            smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
            frames: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Custom widget")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(440.0, 520.0)),
            ..Default::default()
        },
    )
}
