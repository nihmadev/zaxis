//! Frame data and producer identity allocation.

use super::{
    DrawCommand, ExternalImage, MaterialSource, PixelFormat, TextureId, TextureImage,
    TextureOptions, TextureUpdate, Vertex,
};
use crate::Vec2;
use std::{
    ops::Range,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_SOURCE: AtomicU64 = AtomicU64::new(1);

/// A fresh producer identity, also used to tell image caches apart.
pub(crate) fn next_source() -> u64 {
    NEXT_SOURCE.fetch_add(1, Ordering::Relaxed)
}

/// Optional dirty ranges relative to a previously validated geometry revision.
/// Ranges use vertex/index element offsets, not bytes. Backends may ignore this
/// hint and upload the complete frame. Every changed buffer entry must be covered.
#[derive(Clone, Debug, Default)]
pub struct GeometryUpdate {
    pub from_revision: u64,
    pub to_revision: u64,
    pub vertices: Vec<Range<usize>>,
    pub indices: Vec<Range<usize>>,
}

/// Drawing data consumed by a backend. Treat it as immutable during rendering.
///
/// Increment `revision` when vertices, indices, commands, or clipping change.
/// Texture contents have independent revisions on [`TextureImage`].
#[derive(Debug)]
pub struct DrawData {
    /// Shared vertices with logical positions and linear colors.
    pub vertices: Vec<Vertex>,
    /// Global indices into `vertices`.
    pub indices: Vec<u32>,
    /// Draw commands in presentation order.
    pub commands: Vec<DrawCommand>,
    /// Viewport dimensions in logical pixels; finite and positive when rendered.
    pub logical_size: Vec2,
    /// Physical pixels per logical pixel; finite and positive when rendered.
    pub scale_factor: f32,
    /// Geometry revision within this producer.
    pub revision: u64,
    /// Unique producer identity used with `revision` as the geometry cache key.
    /// Keep it stable across frames; constructors allocate it automatically.
    pub source: u64,
    /// Complete current image payloads for textures referenced by commands.
    /// The built-in [`TextureId::WHITE`](super::TextureId::WHITE) needs no payload.
    pub textures: Vec<TextureImage>,
    /// Full current binding policies; omitted IDs preserve legacy behavior.
    /// Independent of geometry revision, including after skipped frames.
    pub texture_options: std::collections::HashMap<TextureId, TextureOptions>,
    /// Byte layout of the payloads; omitted IDs are [`PixelFormat::Rgba8Srgb`]. Independent
    /// of geometry revision, like `texture_options`.
    pub texture_formats: std::collections::HashMap<TextureId, PixelFormat>,
    /// Partial changes of payloads in `textures`, each from the revision it names to the
    /// payload's. Hints only: the payloads stay complete and current, and a backend that
    /// cannot use an update uploads the payload. See [`TextureUpdate`].
    pub texture_updates: Vec<TextureUpdate>,
    /// Host-owned textures the commands draw, which have no payload: the renderer finds each by
    /// ID among those registered with it and checks the size. An ID that is not registered, or
    /// registered with another size, is a render error.
    pub external_textures: Vec<ExternalImage>,
    /// Maximum estimated residency of managed textures. Unmanaged text/white
    /// bindings are pinned. An active set exceeding this limit returns an error.
    pub texture_budget_bytes: u64,
    /// Bound managed binding/view metadata even when many IDs share texel storage.
    pub texture_binding_budget: usize,
    /// Dirty buffer ranges for exactly `from_revision -> to_revision`. None means
    /// a full upload. Clear stale hints when manually editing geometry/revision.
    pub geometry_update: Option<GeometryUpdate>,
    /// The materials commands refer to, each once, with the source to compile. Independent of
    /// geometry revision like textures: a backend that lost its device builds its pipelines
    /// again from the next frame alone.
    pub materials: Vec<MaterialSource>,
    /// Packed per-draw blocks of material commands, addressed by [`MaterialDraw::uniforms`](super::MaterialDraw::uniforms).
    /// Rewritten with `commands`; a backend uploads it again when `revision` changes.
    pub material_uniforms: Vec<u8>,
}

impl DrawData {
    /// Start an independent drawing source with the given logical viewport and DPI.
    /// Reuse this value across frames and increment `revision` after geometry edits.
    pub fn new(logical_size: Vec2, scale_factor: f32) -> Self {
        Self {
            logical_size,
            scale_factor,
            ..Self::default()
        }
    }
}

impl Default for DrawData {
    /// Create an empty source. Set a positive viewport and DPI before rendering.
    fn default() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            commands: Vec::new(),
            logical_size: Vec2::ZERO,
            scale_factor: 0.0,
            revision: 0,
            source: next_source(),
            textures: Vec::new(),
            texture_options: Default::default(),
            texture_formats: Default::default(),
            texture_updates: Vec::new(),
            external_textures: Vec::new(),
            texture_budget_bytes: 256 << 20,
            texture_binding_budget: 1024,
            geometry_update: None,
            materials: Vec::new(),
            material_uniforms: Vec::new(),
        }
    }
}
