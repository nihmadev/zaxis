//! The page side of the browser runner: finding or creating the canvas, sizing it, choosing
//! the graphics API, and keeping the browser's own reactions away from the UI.

use super::CONSUMED;
use crate::app::{browser::blocks_browser_default, RunError, WebBackend, WebOptions};
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Event, HtmlCanvasElement, KeyboardEvent};

/// The canvas the window draws on.
pub(super) struct Page {
    pub(super) canvas: HtmlCanvasElement,
    /// The runner made the canvas cover the whole browser window, so nothing else on the
    /// page can scroll under the pointer.
    covers_window: bool,
}

fn error(message: impl Into<String>) -> RunError {
    RunError::Web(message.into())
}

fn document() -> Result<Document, RunError> {
    web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| error("there is no document"))
}

/// Find the canvas named in the options, or create one in the container (the whole window
/// when there is no container) and let CSS size it.
pub(super) fn acquire(options: &WebOptions) -> Result<Page, RunError> {
    let document = document()?;
    let element = |id: &str| {
        document
            .get_element_by_id(id)
            .ok_or_else(|| error(format!("there is no element with id \"{id}\"")))
    };
    if let Some(id) = &options.canvas_id {
        let canvas = element(id)?
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| error(format!("the element \"{id}\" is not a <canvas>")))?;
        return Ok(Page {
            canvas,
            covers_window: false,
        });
    }
    let host = match &options.container_id {
        Some(id) => element(id)?,
        None => document
            .body()
            .ok_or_else(|| error("the document has no <body>"))?
            .into(),
    };
    let canvas: HtmlCanvasElement = document
        .create_element("canvas")
        .map_err(|_| error("creating the canvas failed"))?
        .unchecked_into();
    let style = canvas.style();
    let covers_window = options.container_id.is_none();
    let layout: &[(&str, &str)] = if covers_window {
        &[("position", "fixed"), ("left", "0"), ("top", "0")]
    } else {
        &[]
    };
    for (name, value) in layout
        .iter()
        .chain(&[("width", "100%"), ("height", "100%")])
    {
        let _ = style.set_property(name, value);
    }
    host.append_child(&canvas)
        .map_err(|_| error("adding the canvas to the page failed"))?;
    Ok(Page {
        canvas,
        covers_window,
    })
}

/// Prepare the canvas once its window exists. The listeners registered here run after
/// winit's own on the same canvas, so they see what the UI made of each event.
pub(super) fn install(page: &Page) {
    let canvas = &page.canvas;
    let style = canvas.style();
    for (name, value) in [
        ("display", "block"),
        ("touch-action", "none"),
        ("outline", "none"),
        ("user-select", "none"),
    ] {
        let _ = style.set_property(name, value);
    }
    let _ = canvas.focus();
    listen(canvas, "contextmenu", |event| event.prevent_default());
    listen(canvas, "keydown", |event| {
        let key: KeyboardEvent = event.unchecked_into();
        let consumed = CONSUMED.with(|consumed| consumed.get());
        if blocks_browser_default(
            &key.key(),
            key.ctrl_key(),
            key.meta_key(),
            key.alt_key(),
            consumed,
        ) {
            key.prevent_default();
        }
    });
    let covers_window = page.covers_window;
    listen(canvas, "wheel", move |event| {
        // A canvas inside a page lets the page scroll unless something in the UI took the wheel.
        if covers_window || CONSUMED.with(|consumed| consumed.get()) {
            event.prevent_default();
        }
    });
}

fn listen(canvas: &HtmlCanvasElement, kind: &str, handler: impl FnMut(Event) + 'static) {
    let closure = Closure::<dyn FnMut(Event)>::new(handler);
    let _ = canvas.add_event_listener_with_callback(kind, closure.as_ref().unchecked_ref());
    closure.forget();
}

/// The wgpu backends to try for the configured choice. `?zaxis_backend=webgl2` in the page
/// URL overrides [`WebBackend::Auto`], to try the fallback without rebuilding.
pub(super) fn backends(configured: WebBackend) -> wgpu::Backends {
    let query = web_sys::window().and_then(|window| window.location().search().ok());
    let from_url = query.as_deref().and_then(WebBackend::from_query);
    match (configured, from_url) {
        (WebBackend::Auto, Some(url)) => url,
        (configured, _) => configured,
    }
    .backends()
}
