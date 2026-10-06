//! The WGSL module of a material: library prelude, generated `Params`, then the user's source.

use super::ParamLayout;

const COMMON: &str = include_str!("../shaders/common.wgsl");
const PRELUDE: &str = include_str!("../shaders/material.wgsl");

/// Where the parts of an assembled module start, in lines counted from 1.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sections {
    pub params_start: u32,
    pub user_start: u32,
}

/// The complete module of `fragment` with `layout`'s parameters, and where its parts begin.
pub(crate) fn assemble(fragment: &str, layout: &ParamLayout) -> (String, Sections) {
    let mut module = String::with_capacity(COMMON.len() + PRELUDE.len() + fragment.len() + 256);
    module.push_str(COMMON);
    module.push('\n');
    module.push_str(PRELUDE);
    module.push('\n');
    let params_start = lines(&module) + 1;
    module.push_str("struct Params {\n");
    if layout.fields.is_empty() {
        // WGSL has no empty structs. Nothing reads this member.
        module.push_str("    z_unused: f32,\n");
    }
    for field in &layout.fields {
        module.push_str(&format!("    {}: {},\n", field.name, field.kind.wgsl()));
    }
    module.push_str("}\n");
    let user_start = lines(&module) + 1;
    module.push_str(fragment);
    (
        module,
        Sections {
            params_start,
            user_start,
        },
    )
}

fn lines(text: &str) -> u32 {
    text.matches('\n').count() as u32
}
