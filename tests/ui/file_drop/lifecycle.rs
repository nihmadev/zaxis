//! Hover and drop over time: repaints, cancelling, windows, odd paths, reading.
use super::*;

#[test]
fn hover_lists_files_in_order_and_cancel_clears_it() {
    let mut s = scene();
    s.point_at(s.seen.outer_rect.center());
    s.context
        .simulate_hover_files(vec![file("b.txt"), file("a.txt")]);
    s.frame();
    assert_eq!(s.seen.global_hovered, ["b.txt", "a.txt"]);
    assert!(s.seen.outer_hover, "the zone highlights while files hover");
    s.context.simulate_hover_cancel();
    s.frame();
    assert!(s.seen.global_hovered.is_empty() && !s.seen.outer_hover);
}

#[test]
fn a_hover_that_changes_nothing_asks_for_no_frame() {
    let mut s = scene();
    let inside = s.seen.outer_rect.center();
    s.context
        .simulate_hover_files_at(vec![file("a.png")], inside);
    s.frame();
    s.frame();
    let settle = Instant::now() + Duration::from_secs(5);
    assert!(!s.context.needs_repaint_at(settle));
    // The browser repeats dragover every few milliseconds with the same files.
    for dx in 0..20 {
        s.context
            .simulate_hover_files_at(vec![file("a.png")], inside + Vec2::new(dx as f32, 0.0));
    }
    assert!(
        !s.context.needs_repaint_at(settle),
        "no busy loop while hovering"
    );
    s.context
        .simulate_hover_files_at(vec![file("a.png")], Vec2::new(700.0, 500.0));
    assert!(
        s.context.needs_repaint_at(settle),
        "leaving the zone is a change"
    );
}

#[test]
fn a_drop_wakes_the_loop_once() {
    let mut s = scene();
    let settle = Instant::now() + Duration::from_secs(5);
    assert!(!s.context.needs_repaint_at(settle));
    s.context.simulate_drop_files(vec![file("a.png")]);
    assert!(s.context.needs_repaint_at(settle));
    s.frame();
    s.frame();
    assert!(!s.context.needs_repaint_at(settle));
}

#[test]
fn a_drop_on_one_window_is_not_seen_by_the_other() {
    let mut a = scene();
    let mut b = scene();
    a.point_at(a.seen.outer_rect.center());
    a.drop_files(vec![file("a.png")]);
    b.frame();
    assert_eq!(a.seen.outer, ["a.png"]);
    assert!(b.seen.outer.is_empty() && b.seen.global.is_empty());
}

#[test]
fn odd_paths_and_directories_never_panic() {
    let dir = std::env::temp_dir();
    let missing = dir.join("zaxis-no-such-file-\u{1F600}");
    let long = dir.join("x".repeat(300));
    let mut s = scene();
    s.point_at(Vec2::new(700.0, 500.0));
    for path in [odd_path(), dir.clone(), missing, long] {
        s.context
            .on_window_event(&WindowEvent::DroppedFile(path.clone()));
        s.frame();
        assert_eq!(s.seen.global.len(), 1, "{path:?}");
    }
    let folder = PickedFile::from_path(&dir);
    assert!(folder.is_dir() && folder.size().is_none());
    assert!(folder.read_blocking().is_err());
    let odd = PickedFile::from_path(odd_path());
    assert!(odd.path().is_some() && odd.read_blocking().is_err());
}

#[test]
fn files_know_their_name_kind_and_type() {
    let f = PickedFile::from_memory("photo.JPG", vec![0_u8; 7]);
    assert_eq!(
        (f.name(), f.extension(), f.mime(), f.size()),
        ("photo.JPG", Some("JPG"), Some("image/jpeg"), Some(7))
    );
    let hidden = PickedFile::from_memory(".hidden", Vec::<u8>::new());
    assert_eq!(hidden.extension(), None);
    assert!(f.path().is_none());
}

fn wait<T>(task: &zaxis::FileTask<T>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while task.is_pending() {
        assert!(
            std::time::Instant::now() < deadline,
            "the task never finished"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn reading_happens_in_the_background_and_arrives_once() {
    let path = std::env::temp_dir().join(format!("zaxis-read-{}.txt", std::process::id()));
    std::fs::write(&path, b"hello").unwrap();
    let mut c = Context::new();
    c.run(|_| {});
    let task = c.read_file(&PickedFile::from_path(&path));
    wait(&task);
    assert!(c.needs_repaint(), "the result asks for a frame");
    c.run(|_| {});
    assert!(!c.needs_repaint(), "and exactly one");
    assert_eq!(task.take().unwrap().unwrap(), b"hello");
    assert!(task.take().is_none());
    let missing = c.read_file(&PickedFile::from_path(path.with_extension("none")));
    wait(&missing);
    assert!(matches!(missing.take(), Some(Err(FileError::Io(_)))));
    std::fs::remove_file(path).ok();
}
