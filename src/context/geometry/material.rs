//! Material draws in the command list: uniform blocks and the sources a frame needs.

use super::MaterialUse;
use crate::{DrawData, MaterialDraw};

/// The block of `element`'s material, built once per element.
pub(super) fn block_of(material: Option<&MaterialUse>) -> Option<Vec<u8>> {
    material.map(MaterialUse::block)
}

/// Whether a command drawn with `last` may also draw `material` with `block`: the same
/// material and the same uniform bytes, or no material on either side.
pub(super) fn same(
    data: &DrawData,
    last: &Option<MaterialDraw>,
    material: Option<&MaterialUse>,
    block: &Option<Vec<u8>>,
) -> bool {
    match (last, material, block) {
        (None, None, _) => true,
        (Some(draw), Some(material), Some(block)) => {
            draw.id == material.id() && uniforms(data, draw) == block.as_slice()
        }
        _ => false,
    }
}

/// Bind `material` for a new command: share the block of the previous command when it is
/// equal, otherwise append it. The source joins the frame's list once.
pub(super) fn bind(
    data: &mut DrawData,
    material: Option<&MaterialUse>,
    block: &Option<Vec<u8>>,
) -> Option<MaterialDraw> {
    let (material, block) = material.zip(block.as_ref())?;
    if !data.materials.iter().any(|s| s.id == material.id()) {
        data.materials.push(material.program.source.clone());
    }
    let previous = data.commands.last().and_then(|c| c.material.clone());
    if let Some(draw) = previous.filter(|d| d.id == material.id() && uniforms(data, d) == block) {
        return Some(draw);
    }
    let start = data.material_uniforms.len() as u32;
    data.material_uniforms.extend_from_slice(block);
    Some(MaterialDraw {
        id: material.id(),
        uniforms: start..data.material_uniforms.len() as u32,
    })
}

fn uniforms<'a>(data: &'a DrawData, draw: &MaterialDraw) -> &'a [u8] {
    &data.material_uniforms[draw.uniforms.start as usize..draw.uniforms.end as usize]
}
