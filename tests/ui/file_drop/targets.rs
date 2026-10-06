//! Which target takes a drop and what it takes.
use super::*;

#[test]
fn the_target_under_the_pointer_receives_the_files_and_the_application_does_not() {
    let mut s = scene();
    s.point_at(s.seen.outer_rect.center());
    s.drop_files(vec![file("a.png"), file("b.txt")]);
    assert_eq!(s.seen.outer, ["a.png", "b.txt"]);
    assert!(s.seen.outer_dropped_flag);
    assert!(
        s.seen.global.is_empty(),
        "claimed files are not also global"
    );
}

#[test]
fn a_drop_beside_every_target_stays_global_for_that_pass_only() {
    let mut s = scene();
    s.point_at(Vec2::new(700.0, 500.0));
    s.drop_files(vec![file("a.png")]);
    assert!(s.seen.outer.is_empty());
    assert_eq!(s.seen.global, ["a.png"]);
    s.frame();
    assert!(s.seen.global.is_empty(), "never delivered twice");
}

#[test]
fn the_innermost_target_is_the_only_one_to_receive() {
    let mut s = scene();
    s.inner = true;
    s.frame();
    s.frame();
    s.point_at(s.seen.inner_rect.center());
    s.drop_files(vec![file("a.png")]);
    assert_eq!(s.seen.inner, ["a.png"]);
    assert!(s.seen.outer.is_empty());
}

#[test]
fn extension_filter_and_limit_decide_what_a_target_takes() {
    let mut s = scene();
    s.outer_filter = Some(FileFilter::new("Images", &["PNG", ".jpg"]));
    s.outer_max = 2;
    s.frame();
    s.frame();
    s.point_at(s.seen.outer_rect.center());
    s.drop_files(vec![
        file("a.png"),
        file("notes.txt"),
        file("b.JPG"),
        file("c.png"),
    ]);
    assert_eq!(s.seen.outer, ["a.png", "b.JPG"]);
}

#[test]
fn a_target_that_turns_every_file_down_does_not_highlight_as_acceptable() {
    let mut s = scene();
    s.outer_filter = Some(FileFilter::new("Images", &["png"]));
    s.frame();
    s.frame();
    s.point_at(s.seen.outer_rect.center());
    s.context.simulate_hover_files(vec![file("notes.txt")]);
    s.frame();
    assert!(s.seen.outer_hover && !s.seen.outer_acceptable);
    s.context.simulate_hover_cancel();
    s.context.simulate_hover_files(vec![file("a.png")]);
    s.frame();
    assert!(s.seen.outer_acceptable);
}

#[test]
fn directories_pass_only_a_filter_that_accepts_them() {
    let dir = PickedFile::directory("photos");
    assert!(dir.is_dir() && dir.extension().is_none());
    assert!(!FileFilter::new("Images", &["png"]).accepts(&dir));
    assert!(!FileFilter::any().accepts(&dir));
    assert!(FileFilter::any().directories(true).accepts(&dir));
}

#[test]
fn a_modal_blocks_the_drop_under_it_and_for_the_application() {
    let mut s = scene();
    s.modal = true;
    s.frame();
    s.frame();
    s.point_at(s.seen.outer_rect.center());
    s.drop_files(vec![file("a.png")]);
    assert!(
        s.seen.outer.is_empty(),
        "the zone under the modal is blocked"
    );
    assert!(
        s.seen.global.is_empty(),
        "so is the application's own handler"
    );
}

#[test]
fn a_lone_target_takes_a_drop_that_comes_without_a_position() {
    let mut s = scene();
    s.pointer_away();
    s.drop_files(vec![file("a.png")]);
    assert_eq!(s.seen.outer, ["a.png"]);
}

#[test]
fn two_candidates_without_a_position_are_a_guess_so_nobody_takes_it() {
    let mut s = scene();
    s.inner = true;
    s.frame();
    s.frame();
    s.pointer_away();
    s.drop_files(vec![file("a.png")]);
    assert!(s.seen.outer.is_empty() && s.seen.inner.is_empty());
    assert_eq!(s.seen.global, ["a.png"]);
}
