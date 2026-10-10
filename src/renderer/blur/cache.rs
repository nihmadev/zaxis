//! Signatures of what an effect reads, to skip the filter when it is unchanged.
//!
//! The backdrop of an effect is everything drawn before it that reaches the area it reads
//! (the blur radius included). Its signature hashes the geometry, textures, materials and
//! clear color of those commands, and the signatures of earlier effects that reach the same
//! area, since their output is part of the backdrop. Content drawn after the effect (the
//! panel's own text, hover, a blinking cursor) is not part of it, so interacting with a panel
//! does not invalidate its blur.

use super::plan::{command_area, Plan};
use crate::{Color, DrawData, TextureId};
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Hasher(u64);

impl Hasher {
    pub fn new(seed: u64) -> Self {
        Self(seed ^ 0xcbf2_9ce4_8422_2325)
    }

    pub fn write(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
    }

    pub fn write_f32s(&mut self, values: &[f32]) {
        for pair in values.chunks(2) {
            let (a, b) = (pair[0].to_bits(), pair.get(1).map_or(0, |v| v.to_bits()));
            self.write(u64::from(a) | u64::from(b) << 32);
        }
    }

    pub fn finish(self) -> u64 {
        self.0
    }
}

struct Contents<'a> {
    data: &'a DrawData,
    revisions: HashMap<TextureId, u64>,
    memo: Vec<Option<u64>>,
}

impl Contents<'_> {
    fn command(&mut self, index: usize) -> u64 {
        if let Some(hash) = self.memo[index] {
            return hash;
        }
        let data = self.data;
        let command = &data.commands[index];
        let mut h = Hasher::new(1);
        h.write(command.texture.0 as u64);
        h.write(self.revisions.get(&command.texture).copied().unwrap_or(0));
        let r = command.clip_rect;
        h.write_f32s(&[r.min.x, r.min.y, r.max.x, r.max.y]);
        h.write(
            u64::from(command.blur.map_or(0, f32::to_bits)) | u64::from(command.scroll_hint) << 40,
        );
        if let Some(material) = &command.material {
            h.write(material.id.0);
            let block = data
                .material_uniforms
                .get(material.uniforms.start as usize..material.uniforms.end as usize)
                .unwrap_or(&[]);
            for chunk in block.chunks(8) {
                let mut word = [0u8; 8];
                word[..chunk.len()].copy_from_slice(chunk);
                h.write(u64::from_le_bytes(word));
            }
        }
        for &i in &data.indices[command.indices.start as usize..command.indices.end as usize] {
            let v = &data.vertices[i as usize];
            h.write_f32s(&v.position);
            h.write_f32s(&v.uv);
            h.write_f32s(&v.color);
        }
        let hash = h.finish();
        self.memo[index] = Some(hash);
        hash
    }
}

/// The signature of every job of `plan`, in order. It also folds in what applies to the
/// whole frame: size, scale and clear color.
pub fn signatures(data: &DrawData, plan: &Plan, size: [u32; 2], clear: Color) -> Vec<u64> {
    let mut contents = Contents {
        data,
        revisions: data.textures.iter().map(|t| (t.id, t.revision)).collect(),
        memo: vec![None; data.commands.len()],
    };
    let mut out: Vec<u64> = Vec::with_capacity(plan.jobs.len());
    for job in &plan.jobs {
        let mut h = Hasher::new(2);
        h.write(u64::from(size[0]) | u64::from(size[1]) << 32);
        h.write(u64::from(data.scale_factor.to_bits()));
        h.write(u64::from(u32::from_le_bytes(clear.0)));
        h.write(u64::from(job.sigma.to_bits()));
        let area = job.pyramid[0];
        h.write(u64::from(area.x0) | u64::from(area.y0) << 32);
        h.write(u64::from(area.x1) | u64::from(area.y1) << 32);
        let b = job.bounds;
        h.write(u64::from(b.x0) | u64::from(b.y0) << 32);
        h.write(u64::from(b.x1) | u64::from(b.y1) << 32);
        for index in 0..job.command {
            let reach = match plan.job_of(index) {
                Some(earlier) => Some(plan.jobs[earlier].bounds),
                None => command_area(data, index, size),
            };
            let Some(reach) = reach else {
                continue;
            };
            if !reach.intersects(area) {
                continue;
            }
            h.write(contents.command(index));
            if let Some(earlier) = plan.job_of(index) {
                h.write(out[earlier]);
            }
        }
        out.push(h.finish());
    }
    out
}

/// One signature for a group of jobs.
pub fn combine(signatures: &[u64]) -> u64 {
    let mut h = Hasher::new(3);
    for s in signatures {
        h.write(*s);
    }
    h.finish()
}
