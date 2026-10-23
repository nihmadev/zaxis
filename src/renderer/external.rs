//! Textures the host owns, sampled by the interface without a copy.
//!
//! The host registers a [`wgpu::Texture`] of the renderer's own device and gets a handle whose
//! [`ExternalImage`] it shows with [`ImageSource::external`](crate::ImageSource::external). The
//! renderer keeps a bind group for it; what the host draws into the texture is what the next
//! frame shows, with no upload. zaxis neither inserts barriers nor owns the queue: the host's
//! writes must be submitted before (or in the same submission as, earlier than) the frame that
//! samples the texture, which is the order of any two passes of one queue.
use super::{
    gpu::Gpu,
    textures::{GpuTexture, Texels, TextureStore},
    Renderer,
};
use crate::{ExternalAlpha, ExternalImage, PixelFormat, TextureFilter, TextureId};
use std::{
    error::Error,
    fmt,
    sync::{Mutex, PoisonError, Weak},
};

/// How a host texture is sampled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ExternalTextureOptions {
    /// Fixed for the registration; an `Image` drawn from it ignores its own filter.
    pub filter: TextureFilter,
    pub alpha: ExternalAlpha,
    pub encoding: ExternalEncoding,
}

/// What the color channels of an 8-bit texture hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ExternalEncoding {
    /// sRGB-encoded values, as video, browser and capture frames are. A texture of a format
    /// without the `Srgb` suffix is sampled through the `Srgb` view of its format, so it
    /// must have been created with that format in `view_formats`.
    #[default]
    Srgb,
    /// Linear values. A texture with an `Srgb` format is sampled through its linear view,
    /// which needs `view_formats` the same way. `Rgba16Float` is always linear.
    Linear,
}

/// Why a texture was not accepted.
#[derive(Debug)]
pub enum ExternalTextureError {
    /// The device is lost; see `notify_device_lost` of the embedded renderer.
    DeviceLost(String),
    /// The texture belongs to another `wgpu::Device` than the renderer's.
    DifferentDevice,
    /// The texture lacks `TEXTURE_BINDING`.
    Usage { missing: wgpu::TextureUsages },
    /// Not a single-sample, single-mip, single-layer 2D texture within the device's limits.
    Shape(String),
    /// A format the interface cannot sample here: only `Rgba8Unorm[Srgb]`, `Bgra8Unorm[Srgb]`
    /// (not on the GL backend) and `Rgba16Float` are accepted.
    Format(String),
    /// The view the requested encoding needs was rejected, typically because the texture was
    /// created without that view format.
    View(String),
}

impl fmt::Display for ExternalTextureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeviceLost(message) => write!(f, "GPU device lost: {message}"),
            Self::DifferentDevice => f.write_str(
                "the texture belongs to another wgpu device than the renderer's; only textures of the same device can be shown",
            ),
            Self::Usage { missing } => write!(f, "the texture needs {missing:?}"),
            Self::Shape(message) => write!(f, "unsupported texture shape: {message}"),
            Self::Format(message) => write!(f, "unsupported texture format: {message}"),
            Self::View(message) => write!(f, "the texture cannot be sampled as asked: {message}"),
        }
    }
}

impl Error for ExternalTextureError {}

/// A host texture registered with a renderer and the textures of its family (siblings and
/// embedded renderers made from it). Dropping the handle, or [`unregister`](Self::unregister),
/// releases the bind group and the renderer's reference to the texture; the host's texture is
/// never destroyed, and it stays valid for the host.
///
/// The handle belongs to the renderer family that registered it: draw data that refers to it
/// is for that family's renderers. Registering again after the texture was recreated (another
/// size or format) gives a new handle and a new `ExternalImage`.
pub struct ExternalTexture {
    image: ExternalImage,
    store: Weak<Mutex<TextureStore>>,
}

impl ExternalTexture {
    /// What to show it with: `ImageSource::external(texture.image())`.
    pub fn image(&self) -> ExternalImage {
        self.image
    }
    pub fn id(&self) -> TextureId {
        self.image.id
    }
    pub fn size(&self) -> [u32; 2] {
        self.image.size
    }
    /// Release the binding now. The same as dropping.
    pub fn unregister(self) {}
}

impl Drop for ExternalTexture {
    fn drop(&mut self) {
        if let Some(store) = self.store.upgrade() {
            store
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .textures
                .remove(&self.image.id);
        }
    }
}

impl fmt::Debug for ExternalTexture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.image.fmt(f)
    }
}

impl Gpu {
    pub(super) fn register_external(
        &mut self,
        texture: &wgpu::Texture,
        options: ExternalTextureOptions,
    ) -> Result<ExternalTexture, ExternalTextureError> {
        use wgpu::TextureFormat as F;
        if let Some(message) = self.lost() {
            return Err(ExternalTextureError::DeviceLost(message));
        }
        if !texture
            .usage()
            .contains(wgpu::TextureUsages::TEXTURE_BINDING)
        {
            return Err(ExternalTextureError::Usage {
                missing: wgpu::TextureUsages::TEXTURE_BINDING,
            });
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        let (width, height) = (texture.width(), texture.height());
        if texture.dimension() != wgpu::TextureDimension::D2
            || texture.sample_count() != 1
            || texture.mip_level_count() != 1
            || texture.depth_or_array_layers() != 1
            || width == 0
            || height == 0
            || width.max(height) > limit
        {
            return Err(ExternalTextureError::Shape(format!(
                "{:?} {width}x{height}, {} layers, {} mips, {} samples; need a single-sample 2D texture of one mip and layer, at most {limit} pixels wide",
                texture.dimension(),
                texture.depth_or_array_layers(),
                texture.mip_level_count(),
                texture.sample_count(),
            )));
        }
        let format = texture.format();
        let view_format = match format {
            F::Rgba8Unorm | F::Rgba8UnormSrgb | F::Bgra8Unorm | F::Bgra8UnormSrgb => {
                if matches!(format, F::Bgra8Unorm | F::Bgra8UnormSrgb) && !self.bgra_native {
                    return Err(ExternalTextureError::Format(format!(
                        "{format:?} cannot be sampled on this backend"
                    )));
                }
                match options.encoding {
                    ExternalEncoding::Srgb => format.add_srgb_suffix(),
                    ExternalEncoding::Linear => format.remove_srgb_suffix(),
                }
            }
            F::Rgba16Float => format,
            other => {
                return Err(ExternalTextureError::Format(format!(
                    "{other:?}; use Rgba8Unorm, Rgba8UnormSrgb, Bgra8Unorm, Bgra8UnormSrgb or Rgba16Float"
                )))
            }
        };
        // Errors of the bind group (a texture of another device, one the layout cannot sample)
        // and of the view format come back as values, not as wgpu panics.
        let bind_group = self.checked_binding(texture, view_format, options.filter)?;
        let store = std::sync::Arc::clone(&self.store);
        let mut guard = store.lock().unwrap_or_else(PoisonError::into_inner);
        guard.next_external += 1;
        let id = TextureId(ExternalImage::FIRST_ID | guard.next_external);
        guard.allocations += 1;
        let allocation = guard.allocations;
        let size = [width, height];
        guard.textures.insert(
            id,
            GpuTexture {
                texture: texture.clone(),
                bind_group,
                size,
                texels: Texels {
                    format: if options.alpha == ExternalAlpha::Opaque {
                        PixelFormat::Rgbx8Srgb
                    } else {
                        PixelFormat::Rgba8Srgb
                    },
                    convert: false,
                },
                revision: 0,
                filter: options.filter,
                managed: false,
                last_used: 0,
                allocation,
                pixels: Weak::new(),
                external: true,
            },
        );
        self.stats.texture_bind_group_creations += 1;
        Ok(ExternalTexture {
            image: ExternalImage {
                id,
                size,
                filter: options.filter,
                alpha: options.alpha,
            },
            store: std::sync::Arc::downgrade(&store),
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn checked_binding(
        &self,
        texture: &wgpu::Texture,
        view_format: wgpu::TextureFormat,
        filter: TextureFilter,
    ) -> Result<wgpu::BindGroup, ExternalTextureError> {
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: (view_format != texture.format()).then_some(view_format),
            ..Default::default()
        });
        let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zaxis external texture binding"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.sampler_for(filter)),
                },
            ],
        });
        match pollster::block_on(scope.pop()) {
            None => Ok(group),
            Some(error) => {
                let message = error.to_string();
                Err(if message.to_lowercase().contains("device") {
                    ExternalTextureError::DifferentDevice
                } else if view_format != texture.format() {
                    ExternalTextureError::View(message)
                } else {
                    ExternalTextureError::Format(message)
                })
            }
        }
    }

    /// A browser cannot wait for an error scope: a texture of another device or a view format
    /// the texture was not created with is reported by the browser's validation, not here.
    #[cfg(target_arch = "wasm32")]
    fn checked_binding(
        &self,
        texture: &wgpu::Texture,
        view_format: wgpu::TextureFormat,
        filter: TextureFilter,
    ) -> Result<wgpu::BindGroup, ExternalTextureError> {
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            format: (view_format != texture.format()).then_some(view_format),
            ..Default::default()
        });
        Ok(self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("zaxis external texture binding"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.sampler_for(filter)),
                },
            ],
        }))
    }
}

impl Renderer {
    /// Register a texture of this renderer's device to be shown by `Image`s without a copy.
    /// See [`ExternalTexture`] and [`ExternalImage`]; the host writes the texture itself.
    pub fn register_external_texture(
        &mut self,
        texture: &wgpu::Texture,
        options: ExternalTextureOptions,
    ) -> Result<ExternalTexture, ExternalTextureError> {
        self.gpu.register_external(texture, options)
    }
}
