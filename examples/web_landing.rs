//! A landing page that runs in a native window and, built for `wasm32-unknown-unknown`,
//! on a browser canvas. The UI code is the same in both. See docs/content/docs/web.mdx for
//! the browser build.

#[path = "web_landing/features.rs"]
mod features;
#[path = "web_landing/footer.rs"]
mod footer;
#[path = "web_landing/hero.rs"]
mod hero;
#[path = "web_landing/page.rs"]
mod page;
#[path = "web_landing/reveal.rs"]
mod reveal;
#[path = "web_landing/scroll.rs"]
mod scroll;

use zaxis::winit::{dpi::LogicalSize, window::Window};
use zaxis::RunOptions;

fn main() -> Result<(), zaxis::RunError> {
    let options = RunOptions {
        window_attributes: Window::default_attributes()
            .with_title("zaxis")
            .with_inner_size(LogicalSize::new(1100.0, 720.0)),
        ..Default::default()
    };
    zaxis::run_with_options(page::Landing::new(smoke_test()), options)
}

/// `--smoke-test` presents a few frames and exits; only the desktop build has arguments.
fn smoke_test() -> bool {
    std::env::args().any(|argument| argument == "--smoke-test")
}
