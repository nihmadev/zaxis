//! Parsing and validating an assembled material module before any device sees it.
//!
//! This runs the same front end and validator the GPU backends run, so a material that passes
//! here compiles on every backend, WebGL2 included. A module is rejected when it declares
//! resources, overrides or entry points of its own: the binding set of a material is fixed.

use super::{assemble::Sections, MaterialError, MaterialErrorKind, ParamLayout};
use std::{collections::BTreeSet, error::Error, sync::Arc};
use wgpu::naga::{
    self,
    front::wgsl,
    valid::{Capabilities, ValidationFlags, Validator},
    ShaderStage, TypeInner,
};

/// `(group, binding)` of every resource the prelude declares.
const BINDINGS: [(u32, u32); 7] = [(0, 0), (1, 0), (1, 1), (2, 0), (2, 1), (2, 2), (3, 0)];

/// Check `wgsl`, assembled with `sections` for `layout`, and say what is wrong in terms of the
/// author's own source.
pub(crate) fn validate(
    wgsl_source: &str,
    sections: Sections,
    layout: &ParamLayout,
    label: &Arc<str>,
) -> Result<(), MaterialError> {
    let locate = |line: u32, column: u32, kind: MaterialErrorKind, message: String| {
        if line == 0 {
            // The front end gave no position for this error.
            MaterialError::new(kind, message)
        } else if line >= sections.user_start {
            MaterialError::new(kind, message).at(line - sections.user_start + 1, column)
        } else if line >= sections.params_start {
            MaterialError::new(
                MaterialErrorKind::Schema,
                format!("the generated `Params` struct is invalid: {message}"),
            )
        } else {
            MaterialError::new(
                kind,
                format!(
                    "the source must define `fn material(in: MaterialInput, p: Params) -> vec4<f32>` \
                     and use only what the prelude provides: {message}"
                ),
            )
        }
    };
    let module = wgsl::parse_str(wgsl_source).map_err(|error| {
        let (line, column) = error
            .location(wgsl_source)
            .map_or((0, 0), |l| (l.line_number, l.line_position));
        locate(
            line,
            column,
            MaterialErrorKind::Syntax,
            error.message().to_owned(),
        )
    });
    let module = module.map_err(|error| error.labeled(label))?;
    check_interface(&module).map_err(|error| error.labeled(label))?;
    Validator::new(ValidationFlags::all(), Capabilities::empty())
        .validate(&module)
        .map_err(|error| {
            let (line, column) = error
                .location(wgsl_source)
                .map_or((0, 0), |l| (l.line_number, l.line_position));
            locate(line, column, MaterialErrorKind::Validation, chain(&error))
        })
        .map_err(|error| error.labeled(label))?;
    check_params(&module, layout).map_err(|error| error.labeled(label))
}

fn forbidden(message: &str) -> MaterialError {
    MaterialError::new(MaterialErrorKind::Forbidden, message)
}

/// Only the prelude's resources and its two entry points may exist.
fn check_interface(module: &naga::Module) -> Result<(), MaterialError> {
    if !module.overrides.is_empty() {
        return Err(forbidden("a material cannot declare `override` constants"));
    }
    let bound: BTreeSet<_> = module
        .global_variables
        .iter()
        .filter_map(|(_, v)| v.binding.as_ref().map(|b| (b.group, b.binding)))
        .collect();
    if bound != BINDINGS.into_iter().collect() {
        return Err(forbidden(
            "a material cannot declare resources: textures, samplers and uniforms are fixed by the library",
        ));
    }
    let entries: Vec<_> = module
        .entry_points
        .iter()
        .map(|e| (e.name.as_str(), e.stage))
        .collect();
    if entries
        != [
            ("vs_main", ShaderStage::Vertex),
            ("fs_material", ShaderStage::Fragment),
        ]
    {
        return Err(forbidden(
            "a material cannot declare entry points: it provides `material` only",
        ));
    }
    Ok(())
}

/// The `Params` struct as the front end laid it out must be the one the CPU packs.
fn check_params(module: &naga::Module, layout: &ParamLayout) -> Result<(), MaterialError> {
    let internal = |message: String| MaterialError::new(MaterialErrorKind::Schema, message);
    let Some(TypeInner::Struct { members, .. }) = module
        .types
        .iter()
        .find(|(_, t)| t.name.as_deref() == Some("Params"))
        .map(|(_, t)| &t.inner)
    else {
        return Err(internal("the module has no `Params` struct".into()));
    };
    for (field, member) in layout.fields.iter().zip(members) {
        if member.offset as usize != field.offset {
            return Err(internal(format!(
                "parameter `{}` sits at byte {} in the shader but {} in the packed block",
                field.name, member.offset, field.offset
            )));
        }
    }
    Ok(())
}

/// An error with its causes: validation errors say where in a nested path they are.
fn chain(error: &(dyn Error + 'static)) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut next = Some(error);
    while let Some(e) = next {
        let text = e.to_string();
        if parts.last() != Some(&text) {
            parts.push(text);
        }
        next = e.source();
    }
    parts.join(": ")
}
