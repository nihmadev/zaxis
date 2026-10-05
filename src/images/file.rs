//! Reading an encoded image from a file path. The browser has no file system.

use super::{ImageError, ImageLimits};
use std::path::Path;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn read(path: &Path, limits: &ImageLimits) -> Result<Vec<u8>, ImageError> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).map_err(|e| ImageError(format!("{}: {e}", path.display())))?;
    let length = file
        .metadata()
        .map_err(|e| ImageError(e.to_string()))?
        .len();
    if length > limits.max_encoded_bytes as u64 {
        return Err(ImageError("encoded image exceeds byte limit".into()));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length as usize)
        .map_err(|e| ImageError(e.to_string()))?;
    // A concurrently growing file cannot escape the limit.
    (&mut file)
        .take(limits.max_encoded_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| ImageError(e.to_string()))?;
    Ok(bytes)
}

#[cfg(target_arch = "wasm32")]
pub(super) fn read(path: &Path, _limits: &ImageLimits) -> Result<Vec<u8>, ImageError> {
    Err(ImageError(format!(
        "{}: loading images by file path is unavailable in the browser;          fetch the bytes and use ImageSource::encoded",
        path.display()
    )))
}
