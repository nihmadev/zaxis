//! The browser clipboard.
//!
//! A page cannot read the clipboard synchronously, and may write it only from a user
//! gesture, so the document's own events carry the data:
//!
//! - **Paste**: the `paste` event delivers the text; [`text`](ClipboardBackend::text) hands out
//!   the one that arrived with the key press being handled. Nothing is read asynchronously.
//! - **Copy and cut**: the text is written with `navigator.clipboard.writeText`, and the
//!   `copy`/`cut` event that follows the same key press also receives it through
//!   `clipboardData`, which needs no permission. Either path completes the copy.
//!
//! A refused write is reported on the next frame, unless the event path delivered the text.

use super::{ClipboardBackend, ClipboardError};
use crate::time::Instant;
use js_sys::{Function, Promise, Reflect};
use std::{cell::RefCell, time::Duration};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};

/// How long text from a copy or paste event stays meaningful: one key press.
const FRESH: Duration = Duration::from_millis(750);

#[derive(Default)]
struct State {
    pasted: Option<(String, Instant)>,
    copied: Option<(String, Instant)>,
    error: Option<ClipboardError>,
    installed: bool,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::default();
}

/// Talks to the document through [`install`]ed listeners and `navigator.clipboard`.
pub(crate) struct WebClipboard;

impl ClipboardBackend for WebClipboard {
    fn text(&mut self) -> Result<String, ClipboardError> {
        STATE
            .with(|state| state.borrow_mut().pasted.take())
            .filter(|(_, at)| at.elapsed() <= FRESH)
            .map(|(text, _)| text)
            .ok_or(ClipboardError::ContentNotAvailable)
    }

    fn set_text(&mut self, text: String) -> Result<(), ClipboardError> {
        STATE.with(|state| state.borrow_mut().copied = Some((text.clone(), Instant::now())));
        if let Err(error) = write_text(&text) {
            fail(error);
        }
        Ok(())
    }

    fn take_error(&mut self) -> Option<ClipboardError> {
        STATE.with(|state| state.borrow_mut().error.take())
    }
}

/// Listen for the document's clipboard events. Idempotent; the runner calls it at startup,
/// before the first key press that could paste.
pub(crate) fn install() {
    if STATE.with(|state| std::mem::replace(&mut state.borrow_mut().installed, true)) {
        return;
    }
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    listen(&document, "paste", |event| {
        let text = event
            .clipboard_data()
            .and_then(|data| data.get_data("text/plain").ok())
            .filter(|text| !text.is_empty());
        if let Some(text) = text {
            STATE.with(|state| state.borrow_mut().pasted = Some((text, Instant::now())));
        }
    });
    for kind in ["copy", "cut"] {
        listen(&document, kind, |event| {
            let fresh = STATE.with(|state| {
                state
                    .borrow()
                    .copied
                    .clone()
                    .filter(|(_, at)| at.elapsed() <= FRESH)
            });
            let delivered = fresh.is_some_and(|(text, _)| {
                event
                    .clipboard_data()
                    .is_some_and(|data| data.set_data("text/plain", &text).is_ok())
            });
            if delivered {
                event.prevent_default();
                STATE.with(|state| state.borrow_mut().error = None);
            }
        });
    }
}

fn listen(
    document: &web_sys::Document,
    kind: &str,
    handler: impl FnMut(web_sys::ClipboardEvent) + 'static,
) {
    let closure = Closure::<dyn FnMut(web_sys::ClipboardEvent)>::new(handler);
    let _ = document.add_event_listener_with_callback(kind, closure.as_ref().unchecked_ref());
    closure.forget();
}

fn fail(error: ClipboardError) {
    STATE.with(|state| state.borrow_mut().error = Some(error));
}

fn describe(error: &JsValue) -> String {
    Reflect::get(error, &"message".into())
        .ok()
        .and_then(|message| message.as_string())
        .or_else(|| error.as_string())
        .unwrap_or_else(|| "the browser refused".to_owned())
}

/// Start `navigator.clipboard.writeText`. Its outcome arrives later through [`fail`].
fn write_text(text: &str) -> Result<(), ClipboardError> {
    let unavailable = |reason: &str| ClipboardError::Unavailable(reason.to_owned());
    let navigator = web_sys::window()
        .ok_or_else(|| unavailable("no window"))?
        .navigator();
    let clipboard = Reflect::get(&navigator, &"clipboard".into())
        .ok()
        .filter(|clipboard| !clipboard.is_undefined() && !clipboard.is_null())
        .ok_or_else(|| {
            unavailable("navigator.clipboard needs a secure page (HTTPS or localhost)")
        })?;
    let write = Reflect::get(&clipboard, &"writeText".into())
        .ok()
        .and_then(|write| write.dyn_into::<Function>().ok())
        .ok_or_else(|| unavailable("navigator.clipboard.writeText is missing"))?;
    let promise = write
        .call1(&clipboard, &JsValue::from_str(text))
        .map_err(|error| ClipboardError::Unavailable(describe(&error)))?
        .dyn_into::<Promise>()
        .map_err(|_| unavailable("writeText returned no promise"))?;
    wasm_bindgen_futures::spawn_local(async move {
        if let Err(error) = wasm_bindgen_futures::JsFuture::from(promise).await {
            fail(ClipboardError::Unavailable(describe(&error)));
        }
    });
    Ok(())
}
