//! What a frame asks of the backdrop filter: one job per effect command, the regions each
//! pyramid level must hold, and the grouping of jobs that can share one set of passes.
//!
//! An effect reads the canvas as it is when the effect is drawn. A later effect can be
//! computed together with an earlier one when nothing drawn in between touches the area it
//! reads, so the snapshot taken at the start of the group is still correct for it; otherwise
//! it starts a new group, which sees the earlier one's output (a blur over a blur).

use super::{
    area::Area,
    params::{self, Filter},
};
use crate::{renderer::viewport::scissor, DrawData, Rect, Vec2};
use std::ops::Range;
use winit::dpi::PhysicalSize;

/// Most effect commands one frame may carry.
pub const MAX_EFFECTS: usize = 256;
/// Open areas a group remembers before merging them into their union.
const DIRTY_LIMIT: usize = 32;

/// One effect command to filter.
#[derive(Clone, Debug)]
pub struct Job {
    pub command: usize,
    /// Sigma in window pixels.
    pub sigma: f32,
    /// What the composite writes, window pixels.
    pub bounds: Area,
    pub filter: Filter,
    /// The part of each level (0..=level) that must be computed; `pyramid[0]` is also what
    /// is copied from the canvas.
    pub pyramid: Vec<Area>,
    /// The horizontally filtered rows at the last level.
    pub scratch: Area,
    /// The filtered texels at the last level that the composite reads.
    pub result: Area,
}

/// Jobs in draw order and their groups.
#[derive(Debug, Default)]
pub struct Plan {
    pub jobs: Vec<Job>,
    /// Ranges of `jobs`, in order.
    pub batches: Vec<Range<usize>>,
}

impl Plan {
    pub fn job_of(&self, command: usize) -> Option<usize> {
        self.jobs
            .binary_search_by_key(&command, |job| job.command)
            .ok()
    }
}

/// Where the mesh of command `index` lands in the window, in physical pixels.
pub fn command_area(data: &DrawData, index: usize, size: [u32; 2]) -> Option<Area> {
    let command = &data.commands[index];
    if command.indices.is_empty() {
        return None;
    }
    let physical = PhysicalSize::new(size[0], size[1]);
    scissor(command.clip_rect, data.scale_factor, physical).map(Area::from_scissor)
}

/// The area of the mesh of command `index`.
fn mesh_area(data: &DrawData, index: usize, size: [u32; 2]) -> Option<Area> {
    let command = &data.commands[index];
    let (mut min, mut max) = (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY));
    for &i in &data.indices[command.indices.start as usize..command.indices.end as usize] {
        let position = Vec2::from_array(data.vertices[i as usize].position);
        min = min.min(position);
        max = max.max(position);
    }
    if !(min.x <= max.x && min.y <= max.y) {
        return None;
    }
    let bounds = Rect::from_min_max(min, max).intersect(command.clip_rect);
    let physical = PhysicalSize::new(size[0], size[1]);
    scissor(bounds, data.scale_factor, physical).map(Area::from_scissor)
}

/// The regions of every level that `bounds` needs for `filter` in a window of `size`.
pub fn regions(bounds: Area, filter: Filter, size: [u32; 2]) -> (Vec<Area>, Area, Area) {
    let level = filter.level;
    let last = params::level_size(size, level);
    let result = if level == 0 {
        bounds
    } else {
        // Four B-spline taps reach two texels beyond the covered ones.
        bounds.to_level(level).grow(2, 2, last)
    };
    let r = filter.radius;
    let scratch = result.grow(0, r, last);
    let mut pyramid = vec![Area::default(); level as usize + 1];
    pyramid[level as usize] = scratch.grow(r, 0, last);
    for k in (1..=level).rev() {
        pyramid[k as usize - 1] = pyramid[k as usize].finer(params::level_size(size, k - 1));
    }
    (pyramid, scratch, result)
}

/// Jobs and groups of `data` for a window of `size` physical pixels. Effect commands without
/// visible area have no job.
pub fn plan(data: &DrawData, size: [u32; 2]) -> Plan {
    let mut plan = Plan::default();
    let mut dirty: Vec<Area> = Vec::new();
    let mut start = 0;
    for (index, command) in data.commands.iter().enumerate() {
        let Some(blur) = command.blur else {
            if !plan.jobs.is_empty() {
                if let Some(area) = command_area(data, index, size) {
                    dirty.push(area);
                }
            }
            continue;
        };
        let Some(bounds) = mesh_area(data, index, size) else {
            continue;
        };
        let sigma = blur * data.scale_factor;
        let filter = params::filter(sigma);
        let (pyramid, scratch, result) = regions(bounds, filter, size);
        let reads = pyramid[0];
        if plan.jobs.is_empty() || dirty.iter().any(|area| area.intersects(reads)) {
            if !plan.jobs.is_empty() {
                plan.batches.push(start..plan.jobs.len());
            }
            start = plan.jobs.len();
            dirty.clear();
        }
        dirty.push(bounds);
        if dirty.len() > DIRTY_LIMIT {
            let all = dirty.iter().copied().reduce(Area::union).unwrap_or(bounds);
            dirty = vec![all];
        }
        plan.jobs.push(Job {
            command: index,
            sigma,
            bounds,
            filter,
            pyramid,
            scratch,
            result,
        });
    }
    if !plan.jobs.is_empty() {
        plan.batches.push(start..plan.jobs.len());
    }
    plan
}
