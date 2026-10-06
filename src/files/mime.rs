/// A media type from a file extension, for the common formats. `None` when unknown;
/// the content is never sniffed.
pub(super) fn from_extension(extension: &str) -> Option<&'static str> {
    Some(match extension.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "tif" | "tiff" => "image/tiff",
        "svg" => "image/svg+xml",
        "txt" | "log" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "text/javascript",
        "json" => "application/json",
        "toml" => "application/toml",
        "xml" => "application/xml",
        "rs" => "text/x-rust",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        _ => return None,
    })
}
