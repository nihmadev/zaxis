//! Every built-in animation: easing curves, tweens, transitions, springs,
//! compositions, procedural effects and animated layout.
#![forbid(unsafe_code)]
#[path = "animations/common.rs"]
mod common;
#[path = "animations/compose.rs"]
mod compose;
#[path = "animations/easing.rs"]
mod easing;
#[path = "animations/effects.rs"]
mod effects;
#[path = "animations/layout.rs"]
mod layout;
#[path = "animations/path.rs"]
mod path;
#[path = "animations/physics.rs"]
mod physics;
#[path = "animations/reorder.rs"]
mod reorder;
#[path = "animations/springs.rs"]
mod springs;
#[path = "animations/timeline.rs"]
mod timeline;
#[path = "animations/tweens.rs"]
mod tweens;

use zaxis::{vec2, App, Context, Frame, RunOptions, Window};

#[derive(Clone, Copy, Hash, PartialEq)]
enum Tab {
    Easing,
    Curves,
    Tweens,
    Timeline,
    Springs,
    Physics,
    Path,
    Compose,
    Effects,
    Layout,
    Reorder,
}

const TABS: [(Tab, &str); 11] = [
    (Tab::Easing, "Easing"),
    (Tab::Curves, "Curves"),
    (Tab::Tweens, "Tweens"),
    (Tab::Timeline, "Timeline"),
    (Tab::Springs, "Springs"),
    (Tab::Physics, "Physics"),
    (Tab::Path, "Path"),
    (Tab::Compose, "Compose"),
    (Tab::Effects, "Effects"),
    (Tab::Layout, "Layout"),
    (Tab::Reorder, "Reorder"),
];

struct Demo {
    tab: Tab,
    toggled: bool,
    springs: springs::Springs,
    physics: physics::Physics,
    effects: effects::Effects,
    layout: layout::Layout,
    items: reorder::Items,
    time_scale: f32,
    smoke: bool,
    passes: u32,
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        let mut reduced = context.style().motion.reduced_motion;
        Window::new("Animations")
            .default_position(vec2(20.0, 20.0))
            .default_size(vec2(780.0, 600.0))
            .show(context, |ui| {
                ui.tab_bar(&mut self.tab, TABS);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut reduced, "Reduced motion");
                    ui.label("Time scale");
                    ui.slider(&mut self.time_scale, 0.1..=2.0);
                });
                ui.separator();
                match self.tab {
                    Tab::Easing => easing::show(ui),
                    Tab::Curves => easing::show_expressive(ui),
                    Tab::Tweens => tweens::show(ui, &mut self.toggled),
                    Tab::Timeline => timeline::show(ui),
                    Tab::Springs => springs::show(ui, &mut self.springs),
                    Tab::Physics => physics::show(ui, &mut self.physics),
                    Tab::Path => path::show(ui),
                    Tab::Compose => compose::show(ui),
                    Tab::Effects => effects::show(ui, &mut self.effects),
                    Tab::Layout => layout::show(ui, &mut self.layout),
                    Tab::Reorder => reorder::show(ui, &mut self.items),
                }
            });
        if reduced != context.style().motion.reduced_motion
            || self.time_scale != context.style().motion.time_scale
        {
            let mut style = context.style().clone();
            style.motion.reduced_motion = reduced;
            style.motion.time_scale = self.time_scale;
            context.set_style(style);
        }
        if self.smoke {
            self.passes += 1;
            if let Some((tab, _)) = TABS.get(self.passes as usize / 3) {
                self.tab = *tab;
                context.request_repaint();
            } else {
                println!("animations smoke: all sections rendered");
                frame.close();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    zaxis::run_with_options(
        Demo {
            tab: Tab::Easing,
            toggled: false,
            springs: Default::default(),
            physics: Default::default(),
            effects: Default::default(),
            layout: Default::default(),
            items: Default::default(),
            time_scale: 1.0,
            smoke: std::env::args().any(|arg| arg == "--smoke-test"),
            passes: 0,
        },
        RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("Zaxis animations")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(820.0, 660.0)),
            ..RunOptions::default()
        },
    )
}
