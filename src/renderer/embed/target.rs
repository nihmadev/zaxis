//! Checks of the host's target and viewport, and drawing into a texture.

use super::{lock, EmbedError, EmbedLoad, EmbeddedRenderer, PhysicalRect};
use crate::{
    renderer::{backdrop::BlurTarget, blur::Compose, draw::Area, pipeline::Target, viewport},
    DrawData,
};
use std::sync::Arc;

/// `region` lies in a target of `target` pixels and `data` was built for its size.
pub(super) fn check_region(
    region: PhysicalRect,
    target: [u32; 2],
    data: &DrawData,
    limits: wgpu::Limits,
) -> Result<(), EmbedError> {
    if region.is_empty() || !region.fits(target) {
        return Err(EmbedError::InvalidViewport(format!(
            "region {region:?} is empty or outside the {}x{} target",
            target[0], target[1]
        )));
    }
    if region.width.max(region.height) > limits.max_texture_dimension_2d {
        return Err(EmbedError::InvalidViewport(
            "region exceeds the GPU texture limit".to_owned(),
        ));
    }
    let wanted = data.logical_size * data.scale_factor;
    if !wanted.is_finite()
        || (wanted.x - region.width as f32).abs() > 1.0
        || (wanted.y - region.height as f32).abs() > 1.0
    {
        return Err(EmbedError::InvalidViewport(format!(
            "draw data is {:.1}x{:.1} physical pixels, region is {}x{}",
            wanted.x, wanted.y, region.width, region.height
        )));
    }
    Ok(())
}

impl EmbeddedRenderer {
    /// Draw `data` into `region` of `target`, in passes this call begins on `encoder`.
    ///
    /// Unlike [`record`](Self::record) the passes are the renderer's own, so everything works,
    /// including backdrop blur and materials that read the backdrop: with
    /// [`EmbedLoad::Keep`] the host's content in the region is the backdrop. For that the
    /// frame is drawn into an offscreen copy of the region and copied back, and `target`'s
    /// texture needs `COPY_SRC` (and `RENDER_ATTACHMENT`, always); a missing usage is an
    /// [`EmbedError::TargetUsage`] here instead of a wgpu validation error in the host. A
    /// frame without a backdrop effect draws straight into `target` and needs only
    /// `RENDER_ATTACHMENT`. The backdrop of an effect changes every frame, so its
    /// filter runs every frame.
    ///
    /// `target` is a view of the whole first mip level and layer of a single-sample
    /// 2D texture, whose view format is [`attachment_format`](super::EmbedOptions::attachment_format).
    /// `region` defaults to the whole texture; `data` must have been built for its size.
    /// [`EmbedLoad::Clear`] clears the whole texture, not just the region.
    ///
    /// Only writes into `encoder`; the host submits it. Nothing waits for the GPU. The options'
    /// sample count and depth format do not apply: these passes have neither.
    pub fn render_to(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        region: Option<PhysicalRect>,
        load: EmbedLoad,
        data: &DrawData,
    ) -> Result<(), EmbedError> {
        self.check_alive()?;
        let texture = target.texture();
        let format = self.options.attachment_format();
        if texture.sample_count() != 1 {
            return Err(EmbedError::TargetMismatch(
                "render_to draws into a single-sample texture; resolve it first".to_owned(),
            ));
        }
        if texture.format().remove_srgb_suffix() != format.remove_srgb_suffix() {
            return Err(EmbedError::TargetMismatch(format!(
                "texture is {:?}, options describe {:?}",
                texture.format(),
                self.options.format
            )));
        }
        if !texture
            .usage()
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(EmbedError::TargetUsage {
                missing: wgpu::TextureUsages::RENDER_ATTACHMENT,
                reason: "the interface is drawn into the texture",
            });
        }
        let extent = [texture.width(), texture.height()];
        let region = region.unwrap_or(PhysicalRect::whole(extent));
        check_region(region, extent, data, self.gpu.device.limits())?;
        viewport::validate(data)?;
        let blur = data.commands.iter().any(|c| c.blur.is_some());
        let keep = matches!(load, EmbedLoad::Keep);
        if blur && keep && !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
            return Err(EmbedError::TargetUsage {
                missing: wgpu::TextureUsages::COPY_SRC,
                reason: "backdrop effects read what the host drew in the region",
            });
        }
        let set = match &self.texture_pipelines {
            Some(set) => Arc::clone(set),
            None => {
                let set = lock(&self.gpu.pipelines).get(Target::color(format));
                self.texture_pipelines = Some(Arc::clone(&set));
                set
            }
        };
        let clear = match load {
            EmbedLoad::Keep => crate::Color::TRANSPARENT,
            EmbedLoad::Clear(color) => self.clear_color(color),
        };
        let gpu = &mut self.gpu;
        let store = Arc::clone(&gpu.store);
        let mut store = lock(&store);
        store.clock += 1;
        gpu.prepare_textures(&mut store, data)?;
        gpu.prepare_geometry(data, &store)?;
        gpu.prepare_viewport(data, if blur { [0, 0] } else { [region.x, region.y] });
        let materials = gpu.prepare_materials(data, &set.target)?;
        let mut draws = 0;
        if blur {
            let compose = Compose {
                base: keep.then_some(texture),
                base_origin: [region.x, region.y],
                region: Some([region.x, region.y, region.width, region.height]),
                clear_output: (!keep).then(|| linear(clear)),
            };
            let target = BlurTarget {
                output: target,
                format,
                size: region.size(),
                pipelines: &set,
                compose,
            };
            gpu.render_blur(encoder, data, clear, target, &store, &materials);
            draws = data
                .commands
                .iter()
                .filter(|c| !c.indices.is_empty())
                .count() as u64;
        } else {
            gpu.note_no_effects();
            if !(keep && data.indices.is_empty()) {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("zaxis embedded pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if keep {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(linear(clear))
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_viewport(
                    region.x as f32,
                    region.y as f32,
                    region.width as f32,
                    region.height as f32,
                    0.0,
                    1.0,
                );
                let area = Area {
                    origin: [region.x, region.y],
                    size: region.size(),
                };
                draws = gpu.record_commands(&mut pass, data, &store, &materials, &set, area, None);
            }
        }
        drop(store);
        self.gpu.stats.draw_calls += draws;
        self.gpu.stats.presented_frames += 1;
        self.prepared = None;
        Ok(())
    }

    /// The clear color as the target expects it: premultiplied when it composites with alpha.
    fn clear_color(&self, color: crate::Color) -> crate::Color {
        if self.options.alpha != super::EmbedAlpha::Premultiplied {
            return color;
        }
        let [r, g, b, a] = color.0;
        let scale = |c: u8| (u16::from(c) * u16::from(a) / 255) as u8;
        crate::Color([scale(r), scale(g), scale(b), a])
    }
}

fn linear(color: crate::Color) -> wgpu::Color {
    let c = color.linear();
    wgpu::Color {
        r: f64::from(c[0]),
        g: f64::from(c[1]),
        b: f64::from(c[2]),
        a: f64::from(c[3]),
    }
}
