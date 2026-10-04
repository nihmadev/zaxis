use zaxis::{icons, vec2, App, Color, Context, Frame, Image, ScrollArea, Slider, TextEdit, Window};

macro_rules! gallery {
    ($($name:ident),* $(,)?) => { &[$(&icons::$name),*] };
}

static ICONS: &[&icons::Icon] = gallery![
    USER,
    USERS,
    HOUSE,
    SETTINGS,
    SEARCH,
    BELL,
    MAIL,
    HEART,
    STAR,
    BOOKMARK,
    CALENDAR,
    CLOCK,
    FILE,
    FOLDER,
    FOLDER_OPEN,
    SAVE,
    DOWNLOAD,
    UPLOAD,
    TRASH,
    COPY,
    CLIPBOARD,
    PENCIL,
    PLUS,
    MINUS,
    X,
    CHECK,
    CHEVRON_LEFT,
    CHEVRON_RIGHT,
    CHEVRON_UP,
    CHEVRON_DOWN,
    ARROW_LEFT,
    ARROW_RIGHT,
    REFRESH_CW,
    MENU,
    ELLIPSIS,
    FUNNEL,
    EYE,
    EYE_OFF,
    LOCK,
    KEY,
    LINK,
    SHARE_2,
    PLAY,
    PAUSE,
    SKIP_FORWARD,
    VOLUME_2,
    MIC,
    CAMERA,
    IMAGE,
    MAP_PIN,
    GLOBE,
    WIFI,
    BATTERY,
    SUN,
    MOON,
    CLOUD,
    ZAP,
    CODE,
    TERMINAL,
    GIT_BRANCH,
    BUG,
    CIRCLE_ALERT,
    INFO,
    CIRCLE_QUESTION_MARK,
];

struct Gallery {
    filter: String,
    size: f32,
    tint: Color,
    selected: &'static icons::Icon,
    smoke: bool,
    passes: usize,
}

impl App for Gallery {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        Window::new("Lucide icons")
            .default_size(vec2(640.0, 560.0))
            .min_size(vec2(360.0, 320.0))
            .show(context, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        TextEdit::new(&mut self.filter)
                            .placeholder("Filter")
                            .width(180.0),
                    );
                    ui.add(
                        Slider::new(&mut self.size, 12.0..=64.0)
                            .text("Size")
                            .width(180.0),
                    );
                    ui.color_picker(&mut self.tint, "Color");
                });
                let cell = self.size + 16.0;
                let columns =
                    ((ui.available_width() / (cell + ui.style().spacing)) as usize).max(1);
                let matches: Vec<&'static icons::Icon> = ICONS
                    .iter()
                    .copied()
                    .filter(|icon| icon.name().contains(self.filter.trim()))
                    .collect();
                let height = (ui.available_height() - 40.0).max(0.0);
                ScrollArea::vertical()
                    .id_source("icons")
                    .max_height(height)
                    .show(ui, |ui| {
                        for row in matches.chunks(columns) {
                            ui.horizontal(|ui| {
                                for &icon in row {
                                    ui.push_id(icon.name(), |ui| {
                                        let response = Image::new(icon)
                                            .size(vec2(cell, cell))
                                            .tint(self.tint)
                                            .interactive(true)
                                            .show(ui)
                                            .response;
                                        if response.clicked() {
                                            self.selected = icon;
                                        }
                                    });
                                }
                            });
                        }
                    });
                ui.muted(format!(
                    "icons::{}  ·  {} shown",
                    self.selected.name().replace('-', "_").to_uppercase(),
                    matches.len()
                ));
            });
        if self.smoke {
            self.passes += 1;
            if self.passes >= 3 {
                frame.close();
            } else {
                context.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Gallery {
            filter: String::new(),
            size: 24.0,
            tint: Color::WHITE,
            selected: &icons::USER,
            smoke: std::env::args().any(|arg| arg == "--smoke-test"),
            passes: 0,
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Icons")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(720.0, 620.0)),
            ..Default::default()
        },
    )
}
