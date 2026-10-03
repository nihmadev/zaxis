use zaxis::{
    vec2, App, Button, Color, Context, Frame, HoverStyle, Padding, Rect, ScrollArea, ScrollStyle,
    Shape, Slider, Text, Window,
};

struct Settings {
    flags: [bool; 48],
    levels: [f32; 48],
    rows: usize,
    selected: usize,
    jump: bool,
    smoke: bool,
    passes: usize,
}
impl App for Settings {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Window::new("Long settings")
            .default_size(vec2(720.0, 650.0)).min_size(vec2(480.0, 450.0))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Jump to selected").clicked() { self.jump = true; }
                    if ui.button("100 / 10,000 rows").clicked() {
                        self.rows = if self.rows == 100 { 10000 } else { 100 };
                        self.selected = self.selected.min(self.rows - 1);
                    }
                });
                ui.muted("Wheel or middle-click either column, then move up/down. Escape stops.");
                let width = ((ui.available_width() - ui.style().spacing) * 0.5).max(0.0);
                let height = (ui.available_height() - 38.0).max(0.0);
                ui.horizontal(|ui| {
                    ScrollArea::vertical().id_source("left").max_width(width).max_height(height).show(ui, |ui| {
                        ui.add(Text::new("Preferences").size(19.0));
                        for index in 0..self.flags.len() {
                            ui.push_id(index, |ui| {
                                ui.checkbox(&mut self.flags[index], format!("Setting {}", index + 1));
                                ui.add(Slider::new(&mut self.levels[index], 0.0..=100.0)
                                    .text("Level").precision(0).width(width - 20.0));
                            });
                        }
                    });
                    ScrollArea::vertical().id_source("right").max_width(width).max_height(height).show(ui, |ui| {
                        ui.add(Text::new("Nested list").size(19.0));
                        ui.muted("Only visible rows are built. Resize or shorten the list to check offset clamping.");
                        let mut list = ScrollArea::vertical().id_source("items").max_height(150.0).style(ScrollStyle::compact());
                        if self.jump {
                            list = list.scroll_to_rect(Rect::from_min_size(vec2(0.0, self.selected as f32 * 26.0), vec2(1.0, 26.0)));
                            self.jump = false;
                        }
                        let output = list.show_rows(ui, 26.0, self.rows, |ui, row| {
                            let response = ui.add(Button::new(format!("Item {}", row + 1))
                                .selected(row == self.selected).padding(Padding::symmetric(6.0, 1.0))
                                .min_size(vec2(ui.available_width(), 24.0)).hover_style(HoverStyle::NONE));
                            if response.clicked() { self.selected = row; }
                        });
                        ui.muted(format!("Built rows {}–{} of {}", output.inner.start + 1, output.inner.end, self.rows));
                        ui.label("Two-axis preview");
                        ScrollArea::both().id_source("canvas").max_height(140.0).content_width(620.0).show(ui, |ui| {
                            let rect = ui.allocate_space(vec2(620.0, 360.0));
                            ui.paint(Shape::rect(rect, Color::gray(57)).corner_radius(6.0));
                            for n in 0..10 {
                                let marker = Rect::from_min_size(rect.min + vec2(n as f32 * 60.0, n as f32 * 30.0), vec2(40.0, 24.0));
                                ui.paint(Shape::rect(marker, Color::gray(110)).corner_radius(4.0));
                            }
                        });
                        for n in 0..20 { ui.label(format!("Additional option {}", n + 1)); }
                    });
                });
                ui.muted(format!("Selected item {} · settings are kept in memory", self.selected + 1));
            });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 2 {
                frame.close();
            } else {
                context.request_repaint();
            }
        }
    }
}
fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Settings {
            flags: [true; 48],
            levels: [50.0; 48],
            rows: 10000,
            selected: 7500,
            jump: false,
            smoke: std::env::args().any(|arg| arg == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — ScrollArea")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(820.0, 760.0)),
            ..Default::default()
        },
    )
}
