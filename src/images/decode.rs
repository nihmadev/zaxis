use super::{source::Source, ImageLimits, ImageStage, ImageTiming};
use image::{DynamicImage, ImageDecoder as _, ImageReader};
use resvg::{tiny_skia, usvg};
use std::{
    fmt,
    io::{Cursor, Read},
    sync::Arc,
    time::Instant,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageError(pub String);
impl fmt::Display for ImageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ImageError {}
impl From<image::ImageError> for ImageError {
    fn from(e: image::ImageError) -> Self {
        Self(e.to_string())
    }
}

/// Output contract for extensions: sRGB RGBA8, straight alpha.
#[derive(Clone, Debug)]
pub struct DecodedImage {
    pub size: [u32; 2],
    pub pixels: Arc<Vec<u8>>,
}
/// An optional content-sniffing decoder. Built-ins run first. Called on worker threads.
/// Extensions must enforce the supplied limits before allocating and never block forever.
pub trait ImageDecoder: Send + Sync + 'static {
    fn decode(
        &self,
        bytes: &[u8],
        limits: &ImageLimits,
    ) -> Option<Result<DecodedImage, ImageError>>;
}

#[derive(Clone)]
pub(super) enum Document {
    Raster(DecodedImage),
    Svg { tree: Arc<usvg::Tree>, bytes: usize },
}
impl Document {
    pub fn size(&self) -> [u32; 2] {
        match self {
            Self::Raster(r) => r.size,
            Self::Svg { tree, .. } => [
                tree.size().width().ceil() as u32,
                tree.size().height().ceil() as u32,
            ],
        }
    }
    pub fn bytes(&self) -> usize {
        match self {
            Self::Raster(r) => r.pixels.len(),
            Self::Svg { bytes, .. } => *bytes,
        }
    }
}

pub(super) fn timed<T>(
    timings: &mut Vec<ImageTiming>,
    stage: ImageStage,
    f: impl FnOnce() -> T,
) -> T {
    let start = Instant::now();
    let result = f();
    timings.push(ImageTiming {
        stage,
        duration: start.elapsed(),
    });
    result
}

pub(super) fn load(
    source: &Source,
    limits: &ImageLimits,
    decoders: &[Arc<dyn ImageDecoder>],
    timings: &mut Vec<ImageTiming>,
) -> Result<Document, ImageError> {
    if let Source::Rgba(size, pixels) = source {
        validate_rgba(*size, pixels, limits)?;
        return Ok(Document::Raster(DecodedImage {
            size: *size,
            pixels: Arc::clone(pixels),
        }));
    }
    let file;
    let bytes = match source {
        Source::Static(bytes) => *bytes,
        Source::Encoded(bytes) => bytes,
        Source::Path(path) => {
            file = timed(
                timings,
                ImageStage::FileIo,
                || -> Result<Vec<u8>, ImageError> {
                    let mut file = std::fs::File::open(path.as_ref())
                        .map_err(|e| ImageError(format!("{}: {e}", path.display())))?;
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
                },
            )?;
            &file
        }
        _ => return Err(ImageError("invalid image handle".into())),
    };
    if bytes.len() > limits.max_encoded_bytes {
        return Err(ImageError("encoded image exceeds byte limit".into()));
    }
    let format = timed(timings, ImageStage::Sniff, || {
        image::guess_format(bytes).ok()
    });
    if let Some(format) = format {
        let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
        let mut decoder_limits = image::Limits::default();
        decoder_limits.max_image_width = Some(limits.max_dimension);
        decoder_limits.max_image_height = Some(limits.max_dimension);
        decoder_limits.max_alloc = Some(limits.max_decoded_bytes as u64);
        reader.limits(decoder_limits);
        let mut decoder = timed(timings, ImageStage::Decode, || reader.into_decoder())?;
        let size = decoder.dimensions();
        limits.check_size([size.0, size.1])?;
        if decoder.total_bytes() > limits.max_decoded_bytes as u64 {
            return Err(ImageError("decoder output exceeds memory limit".into()));
        }
        let orientation = decoder.orientation()?;
        let mut image = timed(timings, ImageStage::Decode, || {
            DynamicImage::from_decoder(decoder)
        })?;
        let pixels = timed(timings, ImageStage::Conversion, || {
            image.apply_orientation(orientation);
            let rgba = image.into_rgba8();
            DecodedImage {
                size: [rgba.width(), rgba.height()],
                pixels: Arc::new(rgba.into_raw()),
            }
        });
        validate_rgba(pixels.size, &pixels.pixels, limits)?;
        return Ok(Document::Raster(pixels));
    }
    if let Ok(xml) = std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes)) {
        if xml.trim_start().starts_with('<') {
            return timed(timings, ImageStage::SvgParse, || parse_svg(xml, limits));
        }
    }
    for decoder in decoders {
        if let Some(result) = timed(timings, ImageStage::Decode, || {
            decoder.decode(bytes, limits)
        }) {
            let decoded = result?;
            validate_rgba(decoded.size, &decoded.pixels, limits)?;
            return Ok(Document::Raster(decoded));
        }
    }
    Err(ImageError(
        "unrecognized image content (PNG/JPEG/WebP/BMP/GIF/TIFF/SVG supported)".into(),
    ))
}

fn parse_svg(xml: &str, limits: &ImageLimits) -> Result<Document, ImageError> {
    if xml.len() > limits.max_svg_bytes {
        return Err(ImageError("SVG exceeds encoded byte limit".into()));
    }
    let doc = usvg::roxmltree::Document::parse_with_options(
        xml,
        usvg::roxmltree::ParsingOptions {
            nodes_limit: limits.max_svg_nodes,
            allow_dtd: false,
            ..Default::default()
        },
    )
    .map_err(|e| ImageError(format!("SVG XML: {e}")))?;
    if doc.root_element().tag_name().name() != "svg" {
        return Err(ImageError("XML root must be svg".into()));
    }
    super::svg_limits::xml(&doc, limits)?;
    // Filters and isolated offscreen groups can allocate large intermediate surfaces.
    // Reject filters rather than pretending their scratch memory is covered by RGBA limits.
    if doc
        .descendants()
        .any(|n| n.has_tag_name("filter") || n.has_tag_name("pattern") || n.has_tag_name("mask"))
    {
        return Err(ImageError(
            "SVG filters, patterns and masks are unsupported by the bounded image loader".into(),
        ));
    }
    static FONTS: std::sync::OnceLock<Arc<usvg::fontdb::Database>> = std::sync::OnceLock::new();
    let fonts = FONTS.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_font_data(include_bytes!("../../assets/Lato-Regular.ttf").to_vec());
        Arc::new(db)
    });
    let options = usvg::Options {
        fontdb: fonts.clone(),
        font_family: "Lato".into(),
        image_href_resolver: usvg::ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..Default::default()
    };
    let tree =
        usvg::Tree::from_xmltree(&doc, &options).map_err(|e| ImageError(format!("SVG: {e}")))?;
    limits.check_size([
        tree.size().width().ceil() as u32,
        tree.size().height().ceil() as u32,
    ])?;
    // Bounded conservative accounting for parsed geometry, not a measured heap size.
    let estimate = xml
        .len()
        .saturating_mul(32)
        .saturating_add(doc.descendants().count() * 512);
    if estimate > limits.cpu_cache_bytes {
        return Err(ImageError("parsed SVG exceeds cache budget".into()));
    }
    Ok(Document::Svg {
        tree: Arc::new(tree),
        bytes: estimate,
    })
}

pub(super) fn rasterize(
    tree: &usvg::Tree,
    size: [u32; 2],
    limits: &ImageLimits,
    timings: &mut Vec<ImageTiming>,
) -> Result<DecodedImage, ImageError> {
    limits.check_size(size)?;
    super::svg_limits::raster(tree, size, limits)?;
    let mut pixmap = timed(timings, ImageStage::SvgRaster, || {
        let mut p = tiny_skia::Pixmap::new(size[0], size[1])
            .ok_or_else(|| ImageError("SVG raster allocation failed".into()))?;
        resvg::render(
            tree,
            tiny_skia::Transform::from_scale(
                size[0] as f32 / tree.size().width(),
                size[1] as f32 / tree.size().height(),
            ),
            &mut p.as_mut(),
        );
        Ok::<_, ImageError>(p)
    })?;
    timed(timings, ImageStage::Conversion, || {
        unpremultiply(pixmap.data_mut())
    });
    Ok(DecodedImage {
        size,
        pixels: Arc::new(pixmap.take()),
    })
}
pub(super) fn unpremultiply(pixels: &mut [u8]) {
    for p in pixels.as_chunks_mut::<4>().0 {
        let a = u32::from(p[3]);
        for c in &mut p[..3] {
            *c = (u32::from(*c) * 255 + a / 2)
                .checked_div(a)
                .unwrap_or(0)
                .min(255) as u8;
        }
    }
}
pub(super) fn validate_rgba(
    size: [u32; 2],
    pixels: &[u8],
    limits: &ImageLimits,
) -> Result<(), ImageError> {
    let count = limits.check_size(size)?;
    if count != pixels.len() {
        return Err(ImageError(
            "RGBA byte count does not match dimensions".into(),
        ));
    }
    Ok(())
}
