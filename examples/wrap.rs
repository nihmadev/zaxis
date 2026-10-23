//! Items that continue on the next line: a filter bar of toggles, removable tags, a field
//! with its action and a menu, and a group of cards of different heights. Resize the
//! window: lines break where the width runs out. `--smoke-test` drives the same view
//! headlessly through `Context::on_input` and checks the model and the geometry.
#![forbid(unsafe_code)]
#[path = "wrap/model.rs"]
mod model;
#[path = "wrap/smoke.rs"]
mod smoke;
#[path = "wrap/view.rs"]
mod view;

use model::Model;
use zaxis::{App, Context, Frame, Padding, Root, Theme};

#[derive(Default)]
pub struct Demo {
    pub model: Model,
    pub seen: view::Seen,
    smoke: bool,
    frames: usize,
}

impl Demo {
    pub fn view(&mut self, context: &mut Context) {
        Root::new().padding(Padding::all(14.0)).show(context, |ui| {
            view::show(ui, &mut self.model, &mut self.seen);
        });
    }
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        context.set_theme(Theme::dark());
        self.view(context);
        if self.smoke {
            self.frames += 1;
            if self.frames > 3 {
                frame.close();
            } else {
                context.request_repaint();
            }
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    let smoke = std::env::args().any(|arg| arg == "--smoke-test");
    if smoke {
        match smoke::run() {
            Ok(()) => println!("wrap smoke: input, model and line geometry verified"),
            Err(error) => {
                eprintln!("wrap smoke failed: {error}");
                std::process::exit(1);
            }
        }
    }
    zaxis::run_with_options(
        Demo {
            smoke,
            ..Default::default()
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Wrap")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(720.0, 640.0)),
            ..Default::default()
        },
    )
}
