//! Which dialog belongs to which window, and what closing a window does to it.
use std::{cell::RefCell, rc::Rc, sync::Arc};
use zaxis::app::dialogs::DialogHost;
use zaxis::{DialogBackend, DialogReply, DialogResult, FileDialog, MemoryDialogs, WindowKey};

/// Shares a recording backend with the host that owns it.
#[derive(Clone, Default)]
struct Shared(Rc<RefCell<MemoryDialogs>>);

impl DialogBackend for Shared {
    fn launch(
        &mut self,
        dialog: &FileDialog,
        parent: Option<&Arc<winit::window::Window>>,
        reply: DialogReply,
    ) {
        self.0.borrow_mut().launch(dialog, parent, reply);
    }
}

fn open(
    host: &mut DialogHost,
    context: &mut zaxis::Context,
    launcher: &str,
    parent: &str,
    key: &str,
) -> zaxis::DialogRequest {
    let request = context.open_dialog(key, FileDialog::open_file());
    for launch in context.take_dialog_launches() {
        host.launch(&launcher.into(), launch, &parent.into(), None);
    }
    request
}

#[test]
fn closing_the_parent_cancels_its_dialog_and_only_its_dialog() {
    let backend = Shared::default();
    let mut host = DialogHost::new(backend.clone());
    let (mut main, mut tool) = (zaxis::Context::new(), zaxis::Context::new());
    let first = open(&mut host, &mut main, "main", "main", "a");
    let second = open(&mut host, &mut tool, "tool", "tool", "b");
    assert_eq!(host.open(), 2);
    host.window_closed(&[WindowKey::new("tool")]);
    assert_eq!(host.open(), 1, "no entry is kept for the closed window");
    assert!(matches!(
        tool.take_dialog_result(&second),
        Some(DialogResult::Cancelled)
    ));
    assert!(
        main.take_dialog_result(&first).is_none(),
        "the other window is untouched"
    );
    assert_eq!(backend.0.borrow().pending(), 1);
}

#[test]
fn a_dialog_modal_for_another_window_is_cancelled_by_that_window_closing() {
    let mut host = DialogHost::new(MemoryDialogs::new());
    let mut main = zaxis::Context::new();
    let request = open(&mut host, &mut main, "main", "settings", "a");
    host.window_closed(&[WindowKey::new("settings")]);
    assert!(matches!(
        main.take_dialog_result(&request),
        Some(DialogResult::Cancelled)
    ));
    assert_eq!(host.open(), 0);
}

#[test]
fn a_dialog_whose_opener_closed_is_forgotten_without_a_leak() {
    let mut host = DialogHost::new(MemoryDialogs::new());
    let mut tool = zaxis::Context::new();
    open(&mut host, &mut tool, "tool", "main", "a");
    host.window_closed(&[WindowKey::new("tool")]);
    assert_eq!(host.open(), 0);
}

#[test]
fn a_dialog_answered_by_the_user_is_no_longer_tracked() {
    let backend = Shared::default();
    let mut host = DialogHost::new(backend.clone());
    let mut main = zaxis::Context::new();
    let request = open(&mut host, &mut main, "main", "main", "a");
    assert!(backend.0.borrow_mut().answer(DialogResult::Cancelled));
    assert_eq!(host.open(), 0);
    host.window_closed(&[WindowKey::new("main")]);
    assert!(matches!(
        main.take_dialog_result(&request),
        Some(DialogResult::Cancelled)
    ));
}
