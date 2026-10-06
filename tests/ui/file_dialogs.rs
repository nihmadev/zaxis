//! Dialog requests through the context and a recording backend: no test opens a real dialog.
use crate::prelude::*;
use std::time::Duration;
use zaxis::{DialogBackend, DialogResult, FileDialog, FileDialogKind, MemoryDialogs};

fn show(context: &mut Context, backend: &mut MemoryDialogs) {
    for launch in context.take_dialog_launches() {
        backend.launch(&launch.dialog, None, launch.reply);
    }
}

#[test]
fn a_dialog_is_opened_once_per_key_while_it_is_pending() {
    let mut c = Context::new();
    let mut backend = MemoryDialogs::new();
    let first = c.open_dialog("open", FileDialog::open_file());
    let again = c.open_dialog("open", FileDialog::open_file());
    assert_eq!(first, again, "the same handle");
    show(&mut c, &mut backend);
    assert_eq!(backend.opened().count(), 1, "one dialog, not two");
    assert!(c.dialog_pending(&first));
    let other = c.open_dialog("save", FileDialog::save_file());
    assert_ne!(first, other);
}

#[test]
fn the_result_arrives_once_and_wakes_the_loop_for_one_frame() {
    let mut c = Context::new();
    let mut backend = MemoryDialogs::new();
    c.run(|_| {});
    let request = c.open_dialog("open", FileDialog::open_files());
    show(&mut c, &mut backend);
    assert!(c.take_dialog_result(&request).is_none(), "still open");
    c.run(|_| {});
    assert!(!c.needs_repaint_at(Instant::now() + Duration::from_secs(5)));
    let files = vec![PickedFile::from_memory("a.txt", vec![1_u8])];
    assert!(backend.answer(DialogResult::Picked(files)));
    assert!(c.needs_repaint(), "an answer wakes the loop");
    c.run(|c| {
        let Some(DialogResult::Picked(files)) = c.take_dialog_result(&request) else {
            panic!("the answer is there");
        };
        assert_eq!(files[0].name(), "a.txt");
        assert!(c.take_dialog_result(&request).is_none(), "exactly once");
    });
    assert!(!c.needs_repaint(), "one frame for one answer");
    // A taken answer frees the key for a new dialog.
    let next = c.open_dialog("open", FileDialog::open_files());
    assert_eq!(next, request);
    assert_eq!(c.take_dialog_launches().len(), 1);
}

#[test]
fn only_the_first_answer_counts() {
    let mut c = Context::new();
    let mut backend = MemoryDialogs::new();
    let request = c.open_dialog("open", FileDialog::open_file());
    let launch = c.take_dialog_launches().remove(0);
    backend.launch(&launch.dialog, None, launch.reply.clone());
    assert!(launch.reply.send(DialogResult::Cancelled));
    assert!(!launch.reply.send(DialogResult::Picked(Vec::new())));
    assert!(matches!(
        c.take_dialog_result(&request),
        Some(DialogResult::Cancelled)
    ));
    assert!(c.take_dialog_result(&request).is_none());
}

#[test]
fn simulated_answers_need_no_backend() {
    let mut c = Context::new();
    let request = c.open_dialog("open", FileDialog::pick_folder());
    c.simulate_dialog_result(&request, DialogResult::Cancelled);
    assert!(matches!(
        c.take_dialog_result(&request),
        Some(DialogResult::Cancelled)
    ));
}

#[test]
fn the_builder_carries_everything_to_the_backend() {
    let dialog = FileDialog::save_file()
        .title("Save as")
        .directory("/tmp")
        .file_name("notes.txt")
        .filter(zaxis::FileFilter::new("Text", &["txt"]))
        .parent("settings");
    assert_eq!(dialog.kind(), FileDialogKind::SaveFile);
    assert_eq!(dialog.title_text(), Some("Save as"));
    assert_eq!(dialog.default_file_name(), Some("notes.txt"));
    assert_eq!(dialog.filters()[0].extensions(), ["txt"]);
    assert_eq!(dialog.parent_window().unwrap().as_str(), "settings");
    assert!(FileDialogKind::OpenFiles.is_multiple());
    assert!(!FileDialogKind::PickFolder.is_multiple());
}
