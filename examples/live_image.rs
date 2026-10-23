//! Live frames: a page stand-in updates the rectangles that changed each step, and only those
//! reach the GPU. `--smoke-test` checks the pixels headlessly.
#![forbid(unsafe_code)]
use std::time::Duration;
use zaxis::{vec2, App, Context, Frame, Image, ImageFit, ImageHandle, ImageSource, Window};

mod live_image {
    pub mod page;
    pub mod smoke;
}
use live_image::{
    page::{Page, SIZE},
    smoke,
};

struct Demo {
    page: Page,
    frame: Option<ImageHandle>,
    running: bool,
}

impl App for Demo {
    fn update(&mut self, context: &mut Context, _frame: &mut Frame<'_>) {
        let handle = *self.frame.get_or_insert_with(|| {
            context
                .load_image(ImageSource::bgra_opaque(SIZE, self.page.pixels.clone()))
                .expect("the page fits the image limits")
        });
        if self.running {
            for region in self.page.step() {
                let (rows, stride) = self.page.rows(region);
                context
                    .update_image_region(handle, region, rows, stride)
                    .expect("regions of the page lie inside the image");
            }
            context.request_repaint_after(Duration::from_millis(16));
        }
        Window::new("Live image")
            .default_size(vec2(680.0, 480.0))
            .show(context, |ui| {
                ui.checkbox(&mut self.running, "Animate");
                ui.add(
                    Image::new(handle)
                        .size(vec2(SIZE[0] as f32, SIZE[1] as f32))
                        .fit(ImageFit::Contain)
                        .alt("The page"),
                );
            });
    }
}

fn main() -> Result<(), zaxis::RunError> {
    if std::env::args().any(|arg| arg == "--smoke-test") {
        if let Err(message) = smoke::run() {
            eprintln!("live_image smoke failed: {message}");
            std::process::exit(1);
        }
        return Ok(());
    }
    zaxis::run(Demo {
        page: Page::new(),
        frame: None,
        running: true,
    })
}
