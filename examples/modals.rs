//! Modal dialogs: confirm a deletion, edit in a form, long scrolling content,
//! nested dialogs, a dangerous action and an edge sheet.
//!
//! Flags: `--open <delete|edit|long|nested|danger|sheet>`, `--theme <dark|light|contrast>`,
//! `--blur`, `--slow`, `--reduced-motion`, `--size WxH`, `--smoke-test`.
use zaxis::{
    vec2, App, Button, ButtonVariant, Context, Frame, MotionStyle, Padding, PresentationMode, Root,
    RunOptions, Theme,
};

#[path = "modals/scenarios.rs"]
mod scenarios;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Palette {
    #[default]
    Dark,
    Light,
    HighContrast,
}

impl Palette {
    const ALL: [(&'static str, Self); 3] = [
        ("Dark", Self::Dark),
        ("Light", Self::Light),
        ("High contrast", Self::HighContrast),
    ];

    fn theme(self) -> Theme {
        match self {
            Self::Dark => Theme::dark(),
            Self::Light => Theme::light(),
            Self::HighContrast => Theme::high_contrast(),
        }
    }
}

pub const CATEGORIES: [&str; 3] = ["Work", "Personal", "Archive"];

pub struct Item {
    pub id: u32,
    pub name: String,
    pub category: usize,
}

#[derive(Default)]
pub struct Model {
    pub items: Vec<Item>,
    pub next_id: u32,
    /// Item the open delete or edit dialog refers to.
    pub target: Option<u32>,
    pub delete_open: bool,
    pub edit_open: bool,
    pub draft_name: String,
    pub draft_category: Option<usize>,
    pub long_open: bool,
    pub nested_open: bool,
    pub advanced_open: bool,
    pub danger_open: bool,
    pub sheet_open: bool,
    pub notifications: bool,
    pub status: String,
    pub palette: Palette,
    pub blur: bool,
}

impl Model {
    fn new() -> Self {
        let names = [
            "Quarterly report",
            "Holiday photos",
            "Tax documents",
            "Old drafts",
        ];
        Self {
            items: names
                .iter()
                .enumerate()
                .map(|(i, name)| Item {
                    id: i as u32,
                    name: (*name).to_owned(),
                    category: i % CATEGORIES.len(),
                })
                .collect(),
            next_id: names.len() as u32,
            notifications: true,
            ..Default::default()
        }
    }

    fn open(&mut self, name: &str) {
        match name {
            "delete" => {
                self.target = self.items.first().map(|item| item.id);
                self.delete_open = true;
            }
            "edit" => scenarios::begin_edit(self, self.items.first().map(|item| item.id)),
            "long" => self.long_open = true,
            "nested" => self.nested_open = true,
            "danger" => self.danger_open = true,
            "sheet" => self.sheet_open = true,
            _ => {}
        }
    }

    #[cfg(test)]
    fn any_open(&self) -> bool {
        self.delete_open
            || self.edit_open
            || self.long_open
            || self.nested_open
            || self.danger_open
            || self.sheet_open
    }
}

struct Demo {
    model: Model,
    motion: Option<MotionStyle>,
    applied: Option<(Palette, bool)>,
    smoke: bool,
    frames: usize,
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        // `Theme` and `Style` are large: build them only when the choice changes.
        let wanted = (self.model.palette, self.model.blur);
        if self.applied != Some(wanted) {
            let mut theme = wanted.0.theme();
            theme.overrides.modal.overlay_blur = Some(if wanted.1 { 10.0 } else { 0.0 });
            theme.overrides.motion = self.motion.clone();
            context.set_theme(theme);
            self.applied = Some(wanted);
        }
        show(context, &mut self.model);
        if self.smoke {
            self.frames += 1;
            let step = self.frames / 12;
            let phase = self.frames % 12;
            let names = ["delete", "edit", "long", "nested", "danger", "sheet"];
            if phase == 1 && step < names.len() {
                self.model.open(names[step]);
            }
            if phase == 8 {
                self.model.delete_open = false;
                self.model.edit_open = false;
                self.model.long_open = false;
                self.model.nested_open = false;
                self.model.advanced_open = false;
                self.model.danger_open = false;
                self.model.sheet_open = false;
            }
            if step >= names.len() {
                frame.close();
            }
            context.request_repaint();
        }
    }
}

fn show(context: &mut Context, model: &mut Model) {
    Root::new().padding(Padding::all(24.0)).show(context, |ui| {
        ui.title("Files");
        ui.muted(model.status.clone());
        ui.horizontal(|ui| {
            for (name, palette) in Palette::ALL {
                let selected = model.palette == palette;
                if ui
                    .add(Button::new(name).selected(selected).id_source(name))
                    .clicked()
                {
                    model.palette = palette;
                }
            }
            ui.checkbox(&mut model.blur, "Blur behind overlay");
        });
        ui.separator();
        scenarios::list(ui, model);
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Terms").clicked() {
                model.long_open = true;
            }
            if ui.button("Project settings").clicked() {
                model.nested_open = true;
            }
            if ui.button("Sheet").clicked() {
                model.sheet_open = true;
            }
            if ui
                .add(Button::new("Delete everything").variant(ButtonVariant::Outline))
                .clicked()
            {
                model.danger_open = true;
            }
        });
        scenarios::dialogs(ui, model);
    });
}

fn main() -> Result<(), zaxis::RunError> {
    let args: Vec<String> = std::env::args().collect();
    let after = |flag: &str| {
        args.iter()
            .position(|arg| arg == flag)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let mut model = Model::new();
    match after("--theme").as_deref() {
        Some("light") => model.palette = Palette::Light,
        Some("contrast") => model.palette = Palette::HighContrast,
        _ => {}
    }
    model.blur = args.iter().any(|arg| arg == "--blur");
    if let Some(name) = after("--open") {
        model.open(&name);
    }
    let size = after("--size")
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some(vec2(w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or(vec2(860.0, 620.0));
    let options = RunOptions {
        window_attributes: zaxis::winit::window::Window::default_attributes()
            .with_title("zaxis — Modals")
            .with_inner_size(zaxis::winit::dpi::LogicalSize::new(size.x, size.y)),
        presentation_mode: PresentationMode::Immediate,
        ..Default::default()
    };
    zaxis::run_with_options(
        Demo {
            model,
            applied: None,
            motion: if args.iter().any(|arg| arg == "--reduced-motion") {
                Some(MotionStyle {
                    reduced_motion: true,
                    ..MotionStyle::default()
                })
            } else if args.iter().any(|arg| arg == "--slow") {
                Some(MotionStyle {
                    time_scale: 0.08,
                    ..MotionStyle::default()
                })
            } else {
                None
            },
            smoke: args.iter().any(|arg| arg == "--smoke-test"),
            frames: 0,
        },
        options,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use zaxis::winit::dpi::PhysicalSize;

    #[test]
    fn every_scenario_opens_and_closes() {
        let mut context = Context::new();
        context.set_viewport(PhysicalSize::new(860, 620), 1.0);
        let mut model = Model::new();
        for name in ["delete", "edit", "long", "nested", "danger", "sheet"] {
            model.open(name);
            for _ in 0..4 {
                context.run(|context| show(context, &mut model));
            }
            assert!(model.any_open(), "{name}");
            model.delete_open = false;
            model.edit_open = false;
            model.long_open = false;
            model.nested_open = false;
            model.danger_open = false;
            model.sheet_open = false;
            for _ in 0..4 {
                context.run(|context| show(context, &mut model));
            }
        }
        assert_eq!(model.items.len(), 4);
    }

    #[test]
    fn confirming_a_deletion_removes_exactly_that_item() {
        use zaxis::winit::{event::ElementState, keyboard::KeyCode};
        let mut context = Context::new();
        context.set_viewport(PhysicalSize::new(860, 620), 1.0);
        let mut model = Model::new();
        let run = |context: &mut Context, model: &mut Model| {
            for _ in 0..4 {
                context.run(|context| show(context, model));
            }
        };
        let key = |context: &mut Context, model: &mut Model, code| {
            context.on_key_event(code, ElementState::Pressed, false);
            run(context, model);
            context.on_key_event(code, ElementState::Released, false);
            run(context, model);
        };
        model.open("delete");
        run(&mut context, &mut model);
        // Cancel has focus; Tab reaches Delete.
        key(&mut context, &mut model, KeyCode::Tab);
        key(&mut context, &mut model, KeyCode::Enter);
        assert!(!model.delete_open);
        assert_eq!(model.items.len(), 3);
        assert!(model
            .items
            .iter()
            .all(|item| item.name != "Quarterly report"));
        // Cancel with Enter on the safe default removes nothing.
        model.open("delete");
        run(&mut context, &mut model);
        key(&mut context, &mut model, KeyCode::Enter);
        assert_eq!(model.items.len(), 3);
    }
}
