//! Copy, cut and paste through an installed clipboard backend: the same widget code the
//! system and the browser clipboards serve, without touching either.
use super::*;
use zaxis::{ClipboardBackend, ClipboardError, DiagnosticKind};

fn focused_field() -> (Context, String, String) {
    let mut c = setup();
    let mut a = "Кириллица 👩‍💻".to_owned();
    let mut b = String::new();
    draw(&mut c, &mut a, &mut b, true, false);
    key(&mut c, KeyCode::Tab, ModifiersState::empty());
    (c, a, b)
}

#[test]
fn copy_cut_and_paste_go_through_the_installed_backend() {
    let (mut c, mut a, mut b) = focused_field();
    let clipboard = MemoryClipboard::with_text("outside");
    c.set_clipboard(clipboard.clone());
    let text = a.clone();
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    key(&mut c, KeyCode::KeyC, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(clipboard.get().as_deref(), Some(text.as_str()));
    assert_eq!(a, text, "copy keeps the text");
    key(&mut c, KeyCode::KeyX, ModifiersState::CONTROL);
    assert!(draw(&mut c, &mut a, &mut b, true, false)[0].changed());
    assert!(a.is_empty());
    clipboard.set("from the page\r\nsecond");
    key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, "from the pagesecond", "a single line drops line breaks");
    assert!(c.diagnostics().is_empty(), "{:?}", c.diagnostics());
}

#[test]
fn read_only_fields_copy_but_do_not_cut_or_paste() {
    let (mut c, mut a, mut b) = focused_field();
    let clipboard = MemoryClipboard::with_text("outside");
    c.set_clipboard(clipboard.clone());
    let text = a.clone();
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    key(&mut c, KeyCode::KeyX, ModifiersState::CONTROL);
    key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
    assert!(!draw(&mut c, &mut a, &mut b, true, true)[0].changed());
    assert_eq!(a, text);
    key(&mut c, KeyCode::KeyC, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, true);
    assert_eq!(clipboard.get().as_deref(), Some(text.as_str()));
}

#[test]
fn a_refused_clipboard_is_reported_and_never_destroys_text() {
    let (mut c, mut a, mut b) = focused_field();
    let clipboard = MemoryClipboard::with_text("outside");
    clipboard.fail_with(Some("permission denied"));
    c.set_clipboard(clipboard.clone());
    let text = a.clone();
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    key(&mut c, KeyCode::KeyX, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, text, "a cut that could not copy removes nothing");
    key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(a, text);
    let reports: Vec<_> = c
        .diagnostics()
        .iter()
        .filter(|d| d.kind == DiagnosticKind::External)
        .collect();
    assert!(!reports.is_empty());
    assert!(reports[0].message.contains("permission denied"));
}

/// A browser completes writes after the call: the failure arrives with a later frame.
struct LateFailure(Option<ClipboardError>);
impl ClipboardBackend for LateFailure {
    fn text(&mut self) -> Result<String, ClipboardError> {
        Err(ClipboardError::ContentNotAvailable)
    }
    fn set_text(&mut self, _: String) -> Result<(), ClipboardError> {
        self.0 = Some(ClipboardError::Unavailable("write refused".into()));
        Ok(())
    }
    fn take_error(&mut self) -> Option<ClipboardError> {
        self.0.take()
    }
}

#[test]
fn a_write_that_fails_after_returning_is_reported_on_the_next_frame() {
    let (mut c, mut a, mut b) = focused_field();
    c.set_clipboard(LateFailure(None));
    key(&mut c, KeyCode::KeyA, ModifiersState::CONTROL);
    key(&mut c, KeyCode::KeyC, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    draw(&mut c, &mut a, &mut b, true, false);
    assert!(c
        .diagnostics()
        .iter()
        .any(|d| d.kind == DiagnosticKind::External && d.message.contains("write refused")));
    key(&mut c, KeyCode::KeyV, ModifiersState::CONTROL);
    draw(&mut c, &mut a, &mut b, true, false);
    let after = c.diagnostics().len();
    draw(&mut c, &mut a, &mut b, true, false);
    assert_eq!(
        c.diagnostics().len(),
        after,
        "an empty clipboard is not an error"
    );
}
