//! Files from the system: a drop zone, native dialogs, previews that never block the frame.
//! `--smoke-test` runs the same model headlessly with in-memory files and a recording
//! dialog backend; nothing is shown.
#[path = "file_drop/model.rs"]
mod model;
#[path = "file_drop/view.rs"]
mod view;

use model::{Action, Model, Preview};
use zaxis::{
    testing::FileInput, App, Context, DialogBackend, DialogResult, Frame, MemoryDialogs,
    PickedFile, Root,
};

struct Demo {
    model: Model,
}

impl App for Demo {
    fn update(&mut self, c: &mut Context, _frame: &mut Frame<'_>) {
        view::pump(c, &mut self.model);
        Root::new().show(c, |ui| {
            view::toolbar(ui, &mut self.model);
            view::drop_zone(ui, &mut self.model);
            view::list(ui, &mut self.model);
            view::preview(ui, &mut self.model);
        });
    }
}

fn main() -> Result<(), zaxis::RunError> {
    if std::env::args().any(|a| a == "--smoke-test") {
        return smoke();
    }
    zaxis::run_with_options(
        Demo {
            model: Model::new(),
        },
        zaxis::RunOptions {
            window_attributes: zaxis::winit::window::Window::default_attributes()
                .with_title("zaxis — Files")
                .with_inner_size(zaxis::winit::dpi::LogicalSize::new(760.0, 640.0)),
            ..Default::default()
        }
        .with_rounded_corners(true),
    )
}

/// Drive the model through a drop, an open dialog and a save without a window.
fn smoke() -> Result<(), zaxis::RunError> {
    let mut c = Context::new();
    c.set_viewport(zaxis::winit::dpi::PhysicalSize::new(760, 640), 1.0);
    let mut model = Model::new();
    let mut backend = MemoryDialogs::new();
    let frame = |c: &mut Context, model: &mut Model| {
        c.run(|c| {
            model.pump(c);
            Root::new().show(c, |ui| {
                view::toolbar(ui, model);
                view::drop_zone(ui, model);
                view::list(ui, model);
                view::preview(ui, model);
            });
        });
    };
    frame(&mut c, &mut model);
    c.simulate_drop_files(vec![PickedFile::from_memory("a.txt", b"hello".to_vec())]);
    frame(&mut c, &mut model);
    assert_eq!(model.files.len(), 1, "the zone took the drop");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !matches!(model.preview, Preview::Text) {
        frame(&mut c, &mut model);
        assert!(
            std::time::Instant::now() < deadline,
            "the preview never loaded"
        );
    }
    assert_eq!(model.text, "hello");
    model.open_dialog(&mut c, Action::Open);
    for launch in c.take_dialog_launches() {
        backend.launch(&launch.dialog, None, launch.reply);
    }
    let picked = PickedFile::from_memory("b.md", b"# b".to_vec());
    backend.answer(DialogResult::Picked(vec![picked]));
    frame(&mut c, &mut model);
    assert_eq!(model.files.len(), 2, "the dialog's file was added");
    println!("file_drop smoke test passed");
    Ok(())
}
