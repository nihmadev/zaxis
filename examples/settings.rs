use zaxis::{
    vec2, App, Border, Color, ColorPicker, ColorPickerType, Context, Frame, Padding,
    PresentationMode, Root, RunOptions, Shape, Slider, Text,
};

#[path = "settings/chrome.rs"]
mod chrome;

#[derive(Default)]
struct SettingsExample {
    settings: SettingsModel,
    smoke_frames: usize,
    smoke_test: bool,
}

impl App for SettingsExample {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        self.settings.window_maximized = frame.window().is_maximized();
        show_ui(context, &mut self.settings);
        if let Some(action) = self.settings.window_action.take() {
            chrome::apply(action, frame);
        }
        if self.smoke_test {
            self.smoke_frames += 1;
            if self.smoke_frames >= 6 {
                frame.close();
            } else {
                self.settings.section = Section::ALL[self.smoke_frames % Section::ALL.len()];
                self.settings.draft.blur_enabled = self.smoke_frames != 3;
                self.settings.draft.blur_radius = [4.0, 12.0, 32.0][self.smoke_frames % 3];
                context.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    let options = RunOptions {
        window_attributes: zaxis::winit::window::Window::default_attributes()
            .with_title("zaxis — Settings reference")
            .with_decorations(false)
            .with_inner_size(zaxis::winit::dpi::LogicalSize::new(820.0, 780.0))
            .with_min_inner_size(zaxis::winit::dpi::LogicalSize::new(760.0, 720.0)),
        presentation_mode: if std::env::args().any(|arg| arg == "--vsync") {
            PresentationMode::Vsync
        } else {
            PresentationMode::Immediate
        },
    }
    .with_rounded_corners(true);
    zaxis::run_with_options(
        SettingsExample {
            smoke_test: std::env::args().any(|arg| arg == "--smoke-test"),
            settings: SettingsModel {
                section: if std::env::args().any(|arg| {
                    matches!(
                        arg.as_str(),
                        "--open-color-picker" | "--internal-color-picker"
                    )
                }) {
                    Section::Appearance
                } else {
                    Section::General
                },
                internal_color_picker: std::env::args().any(|arg| arg == "--internal-color-picker"),
                picker_open_by_default: std::env::args().any(|arg| arg == "--open-color-picker"),
                ..Default::default()
            },
            ..Default::default()
        },
        options,
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
enum Section {
    #[default]
    General,
    Appearance,
    Notifications,
}

impl Section {
    const ALL: [Self; 3] = [Self::General, Self::Appearance, Self::Notifications];

    fn caption(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Appearance => "Appearance",
            Self::Notifications => "Notifications",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Settings {
    restore_session: bool,
    confirm_exit: bool,
    autosave: bool,
    autosave_minutes: f32,
    rounded_preview: bool,
    corner_radius: f32,
    preview_color: Color,
    blur_enabled: bool,
    blur_radius: f32,
    notifications: bool,
    sound: bool,
    volume: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            restore_session: true,
            confirm_exit: true,
            autosave: true,
            autosave_minutes: 5.0,
            rounded_preview: true,
            corner_radius: 10.0,
            preview_color: Color::rgb(78, 133, 190),
            blur_enabled: true,
            blur_radius: 12.0,
            notifications: true,
            sound: false,
            volume: 50.0,
        }
    }
}

#[derive(Default)]
struct SettingsModel {
    window_maximized: bool,
    window_action: Option<chrome::Action>,
    section: Section,
    internal_color_picker: bool,
    picker_open_by_default: bool,
    draft: Settings,
    applied: Settings,
    message: &'static str,
}

impl SettingsModel {
    fn dirty(&self) -> bool {
        self.draft != self.applied
    }

    fn apply(&mut self) {
        self.applied = self.draft.clone();
        self.message = "Applied to the example's in-memory settings.";
    }

    fn cancel(&mut self) {
        self.draft = self.applied.clone();
        self.message = "Unapplied changes discarded.";
    }

    fn reset(&mut self) {
        self.draft = Settings::default();
        self.message = "Defaults restored to the draft. Apply to commit.";
    }
}

fn show_ui(context: &mut Context, model: &mut SettingsModel) {
    let blur = if model.draft.blur_enabled {
        model.draft.blur_radius
    } else {
        0.0
    };
    if context.style().blur_radius != blur {
        context.set_style(context.style().clone().blur(blur));
    }
    // Neutral backdrop geometry makes the glass effect visible behind the actual UI.
    let size = context.viewport().size();
    for (position, radius, gray) in [
        (vec2(0.18, 0.30), 100.0, 76),
        (vec2(0.76, 0.52), 140.0, 92),
        (vec2(0.40, 0.88), 120.0, 62),
    ] {
        context.paint_background(Shape::Circle {
            center: position * size,
            radius,
            fill: Color::gray(gray),
            border: Border::NONE,
        });
    }
    let before_frame = model.draft.clone();
    Root::new()
        .padding(Padding {
            top: zaxis::TitleBar::HEIGHT + 12.0,
            ..Padding::all(24.0)
        })
        .show(context, |ui| {
            ui.separator();
            ui.add(Text::new("Make it work your way").size(25.0));
            ui.muted("A working reference built with the public zaxis API.");
            ui.tab_bar(
                &mut model.section,
                Section::ALL.map(|section| (section, section.caption())),
            );
            ui.separator();

            // Each page has its own stable scope; values survive section switches.
            let before = model.draft.clone();
            ui.push_id(model.section, |ui| match model.section {
                Section::General => {
                    ui.add(Text::new("Session & saving").size(19.0));
                    ui.muted("Choose how a desktop tool should remember your work.");
                    ui.checkbox(
                        &mut model.draft.restore_session,
                        "Restore the previous session",
                    );
                    ui.checkbox(
                        &mut model.draft.confirm_exit,
                        "Confirm before exiting with unsaved work",
                    );
                    ui.checkbox(&mut model.draft.autosave, "Save automatically");
                    ui.add_enabled(
                        model.draft.autosave,
                        Slider::new(&mut model.draft.autosave_minutes, 1.0..=30.0)
                            .text("Save interval")
                            .precision(0)
                            .suffix(" minutes")
                            .step(1.0)
                            .width(360.0),
                    );
                    ui.muted("The interval is editable while automatic saving is enabled.");
                }
                Section::Appearance => {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.checkbox(&mut model.internal_color_picker, "Internal Color Picker");
                            ui.add(
                                ColorPicker::new(&mut model.draft.preview_color, "Preview color")
                                    .picker_type(if model.internal_color_picker {
                                        ColorPickerType::Internal
                                    } else {
                                        ColorPickerType::Floating
                                    })
                                    .default_open(model.picker_open_by_default)
                                    .width(300.0),
                            );
                        });
                        ui.vertical(|ui| {
                            ui.checkbox(&mut model.draft.rounded_preview, "Rounded preview");
                            ui.add_enabled(
                                model.draft.rounded_preview,
                                Slider::new(&mut model.draft.corner_radius, 0.0..=24.0)
                                    .text("Corner radius")
                                    .precision(0)
                                    .suffix(" px")
                                    .step(1.0)
                                    .width(280.0),
                            );
                            ui.checkbox(&mut model.draft.blur_enabled, "Blur window and controls");
                            ui.add_enabled(
                                model.draft.blur_enabled,
                                Slider::new(&mut model.draft.blur_radius, 0.0..=32.0)
                                    .text("Backdrop blur")
                                    .precision(0)
                                    .suffix(" px")
                                    .step(1.0)
                                    .width(280.0),
                            );
                            let width = ui.available_width();
                            let rect = ui.allocate_space(vec2(width, 48.0));
                            ui.paint(Shape::rect(rect, model.draft.preview_color).corner_radius(
                                if model.draft.rounded_preview {
                                    model.draft.corner_radius
                                } else {
                                    0.0
                                },
                            ));
                        });
                    });
                }
                Section::Notifications => {
                    ui.add(Text::new("Alerts & sound").size(19.0));
                    ui.muted("Dependent controls retain their values while disabled.");
                    ui.checkbox(&mut model.draft.notifications, "Enable notifications");
                    ui.add_enabled_ui(model.draft.notifications, |ui| {
                        ui.checkbox(&mut model.draft.sound, "Play a notification sound");
                        ui.add_enabled(
                            model.draft.sound,
                            Slider::new(&mut model.draft.volume, 0.0..=100.0)
                                .text("Volume")
                                .precision(0)
                                .suffix("%")
                                .step(5.0)
                                .width(360.0),
                        );
                    });
                    ui.muted("Volume is editable when both notifications and sound are enabled.");
                }
            });
            if model.draft != before {
                model.message = "";
            }

            ui.add_space(8.0);
            ui.separator();
            ui.horizontal_aligned(zaxis::Align::Center, |ui| {
                if ui.button("Restore defaults").clicked() {
                    model.reset();
                }
                ui.spacer();
                if ui.button_enabled(model.dirty(), "Cancel changes").clicked() {
                    model.cancel();
                }
                if ui.button_enabled(model.dirty(), "Apply").clicked() {
                    model.apply();
                }
            });
            ui.muted(if model.dirty() {
                "You have unapplied changes."
            } else {
                "No unapplied changes."
            });
            ui.muted(if model.message.is_empty() {
                "Demo only: settings live in memory; no files or OS preferences are changed."
            } else {
                model.message
            });
        });
    model.window_action = chrome::show(context, model.window_maximized);
    if model.draft != before_frame {
        context.request_repaint();
    }
}

#[cfg(test)]
#[path = "settings/tests.rs"]
mod tests;
