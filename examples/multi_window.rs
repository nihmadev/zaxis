//! Several native windows: an editor, a declared inspector, a tool palette, a settings
//! window that re-themes every window, notes with a close confirmation, and a window that
//! animates beside windows at rest. `--smoke-test` drives the whole lifecycle and exits.

use zaxis::{
    App, CloseRequested, Context, Frame, OpenOutcome, Theme, WindowError, WindowKey, WindowOptions,
    WindowPlan, WindowStatus,
};

#[path = "multi_window/views.rs"]
mod views;

pub const INSPECTOR: &str = "inspector";
pub const PALETTE: &str = "palette";
pub const SETTINGS: &str = "settings";
pub const NOTES: &str = "notes";
pub const ANIMATION: &str = "animation";

/// Application data, shared by every window. Closing a window drops its UI state (scroll
/// offsets, open headers, focus) but never this: reopening a key shows the same text.
#[derive(Default)]
pub struct Data {
    pub doc: String,
    pub saved_doc: String,
    pub notes: String,
    pub saved_notes: String,
    pub light: bool,
    pub show_inspector: bool,
}

impl Data {
    fn fingerprint(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        (
            &self.doc,
            &self.saved_doc,
            &self.notes,
            &self.saved_notes,
            self.show_inspector,
        )
            .hash(&mut hasher);
        hasher.finish()
    }
    pub fn doc_dirty(&self) -> bool {
        self.doc != self.saved_doc
    }
    pub fn notes_dirty(&self) -> bool {
        self.notes != self.saved_notes
    }
}

/// Which windows are asking "discard unsaved changes?".
#[derive(Default)]
pub struct Confirms {
    pub main: bool,
    pub notes: bool,
}

struct Example {
    data: Data,
    confirms: Confirms,
    smoke: Option<Smoke>,
}

impl App for Example {
    fn update(&mut self, context: &mut Context, frame: &mut Frame<'_>) {
        let before = self.data.fingerprint();
        match frame.window_key().as_str() {
            WindowKey::MAIN => views::main(context, frame, &mut self.data, &mut self.confirms),
            INSPECTOR => views::inspector(context, frame, &mut self.data),
            PALETTE => views::palette(context, &mut self.data),
            SETTINGS => views::settings(context, frame, &mut self.data),
            NOTES => views::notes(context, frame, &mut self.data, &mut self.confirms),
            ANIMATION => views::animation(context),
            _ => {}
        }
        if let Some(smoke) = &mut self.smoke {
            smoke.step(context, frame, &mut self.data);
        }
        // Windows redraw on their own input; data edited in one is shown by the others
        // only if they are asked to draw it.
        if self.data.fingerprint() != before {
            frame.windows().request_repaint_all();
        }
    }

    /// The inspector is declared: it exists while `show_inspector` is set.
    fn windows(&mut self, plan: &mut WindowPlan) {
        plan.window_if(self.data.show_inspector, INSPECTOR, || {
            WindowOptions::new("Inspector")
                .with_inner_size(320.0, 360.0)
                .with_min_inner_size(260.0, 240.0)
        });
    }

    /// A window that cannot be created is reported here; the other windows keep running.
    fn window_failed(&mut self, key: &WindowKey, error: &WindowError) {
        eprintln!("window {key} failed: {error}");
    }

    fn close_requested(&mut self, request: &mut CloseRequested<'_>) {
        match request.window().as_str() {
            WindowKey::MAIN if self.data.doc_dirty() => {
                self.confirms.main = true;
                request.reject();
            }
            NOTES if self.data.notes_dirty() => {
                self.confirms.notes = true;
                request.reject();
            }
            // Closing the declared window withdraws its declaration, as the user intended.
            INSPECTOR => self.data.show_inspector = false,
            _ => {}
        }
    }
}

pub fn open_palette(frame: &mut Frame<'_>) -> OpenOutcome {
    frame.open_window(
        PALETTE,
        WindowOptions::new("Palette")
            .with_inner_size(230.0, 250.0)
            .with_resizable(false)
            .with_always_on_top(true)
            .with_parent(WindowKey::main()),
    )
}

pub fn open_settings(frame: &mut Frame<'_>) -> OpenOutcome {
    frame.open_window(
        SETTINGS,
        WindowOptions::new("Settings").with_inner_size(340.0, 200.0),
    )
}

pub fn open_notes(frame: &mut Frame<'_>) -> OpenOutcome {
    frame.open_window(
        NOTES,
        WindowOptions::new("Notes").with_inner_size(380.0, 320.0),
    )
}

pub fn open_animation(frame: &mut Frame<'_>) -> OpenOutcome {
    frame.open_window(
        ANIMATION,
        WindowOptions::new("Animation").with_inner_size(200.0, 200.0),
    )
}

/// A scripted pass over the lifecycle, run on the real runner with real windows.
struct Smoke {
    phase: usize,
    frames: usize,
    waited: usize,
}

impl Smoke {
    fn waiting(&mut self, ready: bool) -> bool {
        self.waited += 1;
        assert!(
            self.waited < 900,
            "smoke test stalled in phase {}",
            self.phase
        );
        if ready {
            self.phase += 1;
            self.waited = 0;
        }
        ready
    }

    fn step(&mut self, context: &mut Context, frame: &mut Frame<'_>, data: &mut Data) {
        if !frame.is_main() {
            return;
        }
        self.frames += 1;
        context.request_repaint();
        frame.window().request_redraw();
        let notes = WindowKey::new(NOTES);
        match self.phase {
            0 => {
                data.show_inspector = true;
                open_palette(frame);
                open_settings(frame);
                open_notes(frame);
                open_animation(frame);
                self.phase = 1;
            }
            1 => {
                if self.waiting(frame.stats().windows_open == 6) {
                    // A repeated request neither duplicates nor recreates a window.
                    assert_eq!(open_palette(frame), OpenOutcome::AlreadyOpen);
                    frame.windows().set_theme(Theme::light());
                    data.notes.push_str("draft");
                }
            }
            2 => {
                if self.waiting(frame.stats().windows_open == 6) {
                    frame.windows().close(notes);
                }
            }
            3 => {
                if self.waiting(frame.window_status(&notes).is_none()) {
                    assert_eq!(frame.stats().windows_open, 5);
                    open_notes(frame);
                }
            }
            4 => {
                if self.waiting(frame.window_status(&notes) == Some(&WindowStatus::Open)) {
                    assert_eq!(frame.stats().windows_open, 6);
                    assert_eq!(data.notes, "draft", "application data survives a reopen");
                    frame.windows().request_close(SETTINGS);
                }
            }
            5 => {
                if self.waiting(frame.stats().windows_open == 5) {
                    data.doc.push_str("unsaved");
                    frame.windows().request_close(WindowKey::main());
                }
            }
            6 => {
                // The application vetoed the close of the main window: everything stays.
                let settled = self.waited >= 5;
                if self.waiting(settled) {
                    assert!(data.doc_dirty());
                    assert_eq!(frame.stats().windows_open, 5);
                    println!("multi_window smoke test ok after {} frames", self.frames);
                    frame.close();
                }
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), zaxis::RunError> {
    let smoke = std::env::args().any(|arg| arg == "--smoke-test");
    let options = zaxis::RunOptions {
        window_attributes: zaxis::winit::window::Window::default_attributes()
            .with_title("zaxis — Editor")
            .with_inner_size(zaxis::winit::dpi::LogicalSize::new(560.0, 420.0)),
        ..Default::default()
    };
    zaxis::run_with_options(
        Example {
            data: Data::default(),
            confirms: Confirms::default(),
            smoke: smoke.then_some(Smoke {
                phase: 0,
                frames: 0,
                waited: 0,
            }),
        },
        options,
    )
}
