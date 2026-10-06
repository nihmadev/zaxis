//! Files dragged over the canvas. winit reports no file drags in a browser, so the page's
//! own `dragenter`/`dragover`/`dragleave`/`drop` events are listened to here; each is queued
//! and the loop is woken, and the runner delivers the queue to the window's context.
//!
//! `preventDefault` is called for every drag that carries files, which both allows the drop
//! and stops the browser from opening the file.

use super::super::{Runner, UserEvent};
use crate::{app::App, files::PickedFile, Vec2};
use std::cell::RefCell;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{DataTransfer, DragEvent, Event, HtmlCanvasElement};
use winit::event_loop::EventLoopProxy;

pub(super) enum Drag {
    /// Files are over the canvas at `position`, known by type only.
    Hover(Vec<PickedFile>, Vec2),
    Cancel,
    Drop(Vec<PickedFile>, Vec2),
}

thread_local! {
    static QUEUE: RefCell<Vec<Drag>> = const { RefCell::new(Vec::new()) };
}

/// Listen on `canvas`; every drag event wakes the loop through `proxy`.
pub(super) fn install(canvas: &HtmlCanvasElement, proxy: EventLoopProxy<UserEvent>) {
    for kind in ["dragenter", "dragover", "dragleave", "drop"] {
        let (target, proxy) = (canvas.clone(), proxy.clone());
        let handler = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
            let event: DragEvent = event.unchecked_into();
            if let Some(drag) = classify(&target, &event) {
                QUEUE.with(|queue| queue.borrow_mut().push(drag));
                let _ = proxy.send_event(UserEvent::Files);
            }
        });
        let _ = canvas.add_event_listener_with_callback(kind, handler.as_ref().unchecked_ref());
        handler.forget();
    }
}

fn carries_files(transfer: &DataTransfer) -> bool {
    let types = transfer.types();
    (0..types.length()).any(|i| types.get(i).as_string().as_deref() == Some("Files"))
}

fn classify(canvas: &HtmlCanvasElement, event: &DragEvent) -> Option<Drag> {
    let transfer = event.data_transfer()?;
    if !carries_files(&transfer) {
        return None;
    }
    event.prevent_default();
    let rect = canvas.get_bounding_client_rect();
    let position = Vec2::new(
        event.client_x() as f32 - rect.left() as f32,
        event.client_y() as f32 - rect.top() as f32,
    );
    match event.type_().as_str() {
        "dragleave" => Some(Drag::Cancel),
        "drop" => {
            let list = transfer.files()?;
            let files = (0..list.length())
                .filter_map(|i| list.item(i))
                .map(PickedFile::from_web)
                .collect();
            Some(Drag::Drop(files, position))
        }
        _ => {
            transfer.set_drop_effect("copy");
            let items = transfer.items();
            let files = (0..items.length())
                .filter_map(|i| items.get(i))
                .filter(|item| item.kind() == "file")
                .map(|item| PickedFile::placeholder(Some(item.type_())))
                .collect();
            Some(Drag::Hover(files, position))
        }
    }
}

impl<A: App> Runner<A> {
    /// Hand the queued drags to the window's context, in order, and draw if any of them
    /// changed what is shown.
    pub(in crate::app::runner) fn web_files(&mut self) {
        let drags = QUEUE.with(|queue| std::mem::take(&mut *queue.borrow_mut()));
        let Some(slot) = self.slots.values_mut().next() else {
            return;
        };
        let mut repaint = false;
        for drag in drags {
            repaint |= match drag {
                Drag::Hover(files, position) => slot.context.file_hover_replace(files, position),
                Drag::Cancel => slot.context.file_hover_cancelled(),
                Drag::Drop(files, position) => slot.context.file_drop_batch(files, position),
            };
        }
        if repaint {
            slot.context.request_repaint();
            if let Some(native) = slot.native.as_ref().filter(|n| n.visible()) {
                native.window.request_redraw();
            }
        }
    }
}
