//! The application identifier of a window: its own, the run-wide one, or the executable.
use zaxis::app::app_id::resolve;
use zaxis::{RunOptions, WindowOptions};

#[test]
fn a_window_id_beats_the_run_id() {
    assert_eq!(resolve(Some("tools"), Some("editor")), "tools");
    assert_eq!(resolve(None, Some("editor")), "editor");
}

#[test]
fn an_empty_id_counts_as_unset() {
    assert_eq!(resolve(Some(""), Some("editor")), "editor");
}

#[test]
fn the_default_is_the_executable_name() {
    let id = resolve(None, None);
    let exe = std::env::current_exe().unwrap();
    assert_eq!(id, exe.file_stem().unwrap().to_string_lossy());
}

#[test]
fn options_carry_the_id() {
    assert_eq!(RunOptions::default().app_id, None);
    assert_eq!(
        RunOptions::default()
            .with_app_id("editor")
            .app_id
            .as_deref(),
        Some("editor")
    );
    assert_eq!(WindowOptions::new("a").app_id(), None);
    assert_eq!(WindowOptions::new("a").with_app_id("x").app_id(), Some("x"));
}
