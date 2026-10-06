use crate::files::{picked::Source, FileError, TaskSender};
use js_sys::{Array, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{spawn_local, JsFuture};

fn failure(value: JsValue) -> FileError {
    FileError::Io(value.as_string().unwrap_or_else(|| format!("{value:?}")))
}

pub(super) fn read(source: &Source, sender: TaskSender<Vec<u8>>) {
    let Source::Web(file) = source else {
        return sender.complete(Err(FileError::Unsupported("no file to read".into())));
    };
    let promise = file.array_buffer();
    spawn_local(async move {
        let result = JsFuture::from(promise)
            .await
            .map(|buffer| Uint8Array::new(&buffer).to_vec())
            .map_err(failure);
        sender.complete(result);
    });
}

/// A browser saves by downloading: an `<a download>` pointing at a blob of the bytes.
pub(super) fn write(source: &Source, name: &str, bytes: Vec<u8>, sender: TaskSender<()>) {
    if !matches!(source, Source::Download) {
        return sender.complete(Err(FileError::Unsupported(
            "a file chosen in a browser cannot be overwritten".into(),
        )));
    }
    sender.complete(download(name, &bytes).map_err(failure));
}

fn download(name: &str, bytes: &[u8]) -> Result<(), JsValue> {
    let parts = Array::of1(&Uint8Array::from(bytes));
    let blob = web_sys::Blob::new_with_u8_array_sequence(&parts)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| JsValue::from_str("there is no document"))?;
    let link: web_sys::HtmlAnchorElement = document.create_element("a")?.unchecked_into();
    link.set_href(&url);
    link.set_download(name);
    link.click();
    web_sys::Url::revoke_object_url(&url)
}
