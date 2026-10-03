use super::*;

/// Limits are per Context. Worker scratch is additional to the retained CPU budget.
#[derive(Clone, Debug)]
pub struct ImageLimits {
    pub max_encoded_bytes: usize,
    pub max_dimension: u32,
    pub max_decoded_bytes: usize,
    pub max_svg_bytes: usize,
    pub max_svg_nodes: u32,
    pub cpu_cache_bytes: usize,
    pub gpu_cache_bytes: u64,
    pub max_gpu_bindings: usize,
    pub max_entries: usize,
    pub max_pending_jobs: usize,
    pub max_inflight_bytes: usize,
    pub raster_variants: usize,
    pub resize_debounce: Duration,
}
impl Default for ImageLimits {
    fn default() -> Self {
        Self {
            max_encoded_bytes: 16 << 20,
            max_dimension: 8192,
            max_decoded_bytes: 64 << 20,
            max_svg_bytes: 1 << 20,
            max_svg_nodes: 50_000,
            cpu_cache_bytes: 128 << 20,
            gpu_cache_bytes: 128 << 20,
            max_gpu_bindings: 1024,
            max_entries: 512,
            max_pending_jobs: 8,
            max_inflight_bytes: 288 << 20,
            raster_variants: 3,
            resize_debounce: Duration::from_millis(120),
        }
    }
}
impl ImageLimits {
    pub fn check_size(&self, size: [u32; 2]) -> Result<usize, ImageError> {
        if size.contains(&0) || size.iter().any(|s| *s > self.max_dimension) {
            return Err(ImageError(format!(
                "image dimensions {:?} exceed limit {} or are zero",
                size, self.max_dimension
            )));
        }
        let bytes = u64::from(size[0])
            .checked_mul(u64::from(size[1]))
            .and_then(|n| n.checked_mul(4))
            .and_then(|n| usize::try_from(n).ok())
            .ok_or_else(|| ImageError("image dimensions overflow".into()))?;
        if bytes > self.max_decoded_bytes {
            return Err(ImageError("decoded RGBA exceeds memory limit".into()));
        }
        Ok(bytes)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ImageState {
    Loading { size: Option<Vec2> },
    Ready { size: Vec2 },
    Error(ImageError),
}
impl ImageState {
    pub fn size(&self) -> Option<Vec2> {
        match self {
            Self::Loading { size } => *size,
            Self::Ready { size } => Some(*size),
            Self::Error(_) => None,
        }
    }
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageStage {
    FileIo,
    Sniff,
    Decode,
    SvgParse,
    SvgRaster,
    Conversion,
    Downsample,
    Queue,
    Service,
    Lookup,
    Publication,
    FirstReady,
}
#[derive(Clone, Copy, Debug)]
pub struct ImageTiming {
    pub stage: ImageStage,
    pub duration: Duration,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct ImageMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub decodes: u64,
    pub svg_parses: u64,
    pub rasterizations: u64,
    pub downsamplings: u64,
    pub stale_completions: u64,
    pub evictions: u64,
    pub backpressure: u64,
    pub pending_jobs: usize,
    pub cpu_resident_bytes: usize,
}
