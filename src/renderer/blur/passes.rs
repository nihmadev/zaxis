//! Encoding of the filter of one group of effects: the snapshot of the canvas, the pyramid
//! steps, the separable Gaussian at the last level and, for materials, the reconstruction at
//! full resolution. Every pass draws one fullscreen triangle per job inside the job's scissor
//! rectangle, so the cost follows the area of the panels, not the window.

use super::{
    area::Area,
    pipelines::BlurPipelines,
    plan::Job,
    targets::{Set, Tex},
    uniforms::Uniforms,
    FrameStats,
};

/// Dynamic offsets of the filter blocks of one job.
#[derive(Clone, Copy, Debug, Default)]
pub struct Offsets {
    pub horizontal: u32,
    pub vertical: u32,
    pub resolve: u32,
}

/// A group of jobs to filter with their blocks.
pub struct Group<'a> {
    pub jobs: &'a [Job],
    pub offsets: &'a [Offsets],
    /// Offset of a block that no pass reads (pyramid steps).
    pub unused: u32,
    /// Whether the job reads the backdrop as a material, and needs the full-resolution result.
    pub materials: &'a [bool],
}

#[allow(clippy::too_many_arguments)]
fn pass<'a>(
    encoder: &mut wgpu::CommandEncoder,
    pipeline: &wgpu::RenderPipeline,
    output: &wgpu::TextureView,
    uniforms: &Uniforms,
    stats: &mut FrameStats,
    draws: impl IntoIterator<Item = (&'a wgpu::BindGroup, Area, u32)>,
) {
    let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("zaxis backdrop filter pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: output,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    });
    rp.set_pipeline(pipeline);
    let mut any = false;
    for (input, area, offset) in draws {
        if area.is_empty() {
            continue;
        }
        let [x, y, w, h] = area.scissor();
        rp.set_scissor_rect(x, y, w, h);
        rp.set_bind_group(0, input, &[]);
        rp.set_bind_group(1, uniforms.filter_group(), &[offset]);
        rp.draw(0..3, 0..1);
        stats.pixels += area.pixels();
        any = true;
    }
    drop(rp);
    stats.passes += u64::from(any);
}

/// Encode the filter of `group` from `canvas` into `set`.
pub fn filter_group(
    encoder: &mut wgpu::CommandEncoder,
    pipelines: &BlurPipelines,
    uniforms: &Uniforms,
    canvas: &Tex,
    set: &Set,
    group: &Group<'_>,
    stats: &mut FrameStats,
) {
    let snapshot = set.snapshot.as_ref().expect("snapshot");
    for job in group.jobs {
        let area = job.pyramid[0];
        if area.is_empty() {
            continue;
        }
        let origin = wgpu::Origin3d {
            x: area.x0,
            y: area.y0,
            z: 0,
        };
        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                origin,
                ..canvas.texture.as_image_copy()
            },
            wgpu::TexelCopyTextureInfo {
                origin,
                ..snapshot.texture.as_image_copy()
            },
            wgpu::Extent3d {
                width: area.x1 - area.x0,
                height: area.y1 - area.y0,
                depth_or_array_layers: 1,
            },
        );
        stats.copies += 1;
        stats.pixels += area.pixels();
    }
    let deepest = group.jobs.iter().map(|j| j.filter.level).max().unwrap_or(0);
    for k in 1..=deepest {
        let output = &set.level(k).view;
        let source = &set.level(k - 1).group;
        let draws = group
            .jobs
            .iter()
            .filter(|j| j.filter.level >= k)
            .map(|j| (source, j.pyramid[k as usize], group.unused));
        pass(encoder, &pipelines.down, output, uniforms, stats, draws);
    }
    for level in 0..=deepest {
        let jobs: Vec<_> = group
            .jobs
            .iter()
            .zip(group.offsets)
            .filter(|(j, _)| j.filter.level == level)
            .collect();
        if jobs.is_empty() {
            continue;
        }
        let scratch = set.scratch[level as usize].as_ref().expect("scratch");
        let result = set.result[level as usize].as_ref().expect("result");
        let horizontal = jobs
            .iter()
            .map(|(j, o)| (&set.level(level).group, j.scratch, o.horizontal));
        pass(
            encoder,
            &pipelines.gaussian,
            &scratch.view,
            uniforms,
            stats,
            horizontal,
        );
        let vertical = jobs
            .iter()
            .map(|(j, o)| (&scratch.group, j.result, o.vertical));
        pass(
            encoder,
            &pipelines.gaussian,
            &result.view,
            uniforms,
            stats,
            vertical,
        );
    }
    if let Some(resolved) = &set.resolved {
        let draws: Vec<_> = group
            .jobs
            .iter()
            .zip(group.offsets)
            .zip(group.materials)
            .filter(|(_, material)| **material)
            .map(|((j, o), _)| {
                let result = set.result[j.filter.level as usize]
                    .as_ref()
                    .expect("result");
                (&result.group, j.bounds, o.resolve)
            })
            .collect();
        if !draws.is_empty() {
            pass(
                encoder,
                &pipelines.resolve,
                &resolved.view,
                uniforms,
                stats,
                draws,
            );
        }
    }
}
