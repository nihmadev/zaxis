//! Registered materials, keyed by content and shared by every window of an application.

use super::{
    assemble::assemble, compile, layout::MAX_PARAM_BYTES, Material, MaterialError,
    MaterialErrorKind, ParamLayout,
};
use crate::{protocol::MaterialSource, MaterialId, SharedResources};
use std::{collections::HashMap, sync::Arc};

/// Registrations a [`SharedResources`] accepts, valid or not. Past it, new descriptions are
/// rejected with [`MaterialErrorKind::Limit`]: building WGSL from dynamic values would
/// otherwise grow the registry and the renderer's pipelines without end.
pub const MAX_MATERIALS: usize = 256;

/// A registered material: validated source, parameter placement and flags. Failed
/// registrations are kept so the error is produced once, not on every frame.
#[derive(Debug)]
pub(crate) struct MaterialProgram {
    pub source: MaterialSource,
    pub layout: ParamLayout,
    pub reads_texture: bool,
    pub reads_backdrop: bool,
    pub animated: bool,
    pub error: Option<MaterialError>,
    key: String,
}

#[derive(Default)]
pub(crate) struct MaterialRegistry {
    entries: HashMap<MaterialId, Arc<MaterialProgram>>,
}

impl MaterialRegistry {
    /// The program of `material`, compiled on first sight. The same description always
    /// answers with the same program.
    pub fn register(&mut self, material: &Material) -> Result<Arc<MaterialProgram>, MaterialError> {
        let key = material.content_key();
        let mut id = MaterialId(hash(&key));
        // A collision of 64-bit hashes moves the newcomer to the next free id.
        while let Some(known) = self.entries.get(&id) {
            if known.key == key {
                return Ok(Arc::clone(known));
            }
            id = MaterialId(id.0.wrapping_add(1).max(1));
        }
        if self.entries.len() >= MAX_MATERIALS {
            return Err(MaterialError::new(
                MaterialErrorKind::Limit,
                format!("at most {MAX_MATERIALS} materials can be registered"),
            )
            .labeled(&material.label));
        }
        let program = Arc::new(build(id, key, material));
        self.entries.insert(id, Arc::clone(&program));
        Ok(program)
    }

    pub fn get(&self, id: MaterialId) -> Option<Arc<MaterialProgram>> {
        self.entries.get(&id).cloned()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn remove(&mut self, id: MaterialId) -> bool {
        self.entries.remove(&id).is_some()
    }
}

fn build(id: MaterialId, key: String, material: &Material) -> MaterialProgram {
    let (layout, wgsl, error) = match ParamLayout::new(&material.params) {
        Err(error) => (ParamLayout::default(), String::new(), Some(error)),
        Ok(layout) => {
            let (wgsl, sections) = assemble(&material.fragment, &layout);
            let error = compile::validate(&wgsl, sections, &layout, &material.label).err();
            (layout, wgsl, error)
        }
    };
    debug_assert!(layout.size() <= MAX_PARAM_BYTES);
    MaterialProgram {
        source: MaterialSource {
            id,
            label: Arc::clone(&material.label),
            wgsl: Arc::from(wgsl),
        },
        layout,
        reads_texture: material.reads_texture,
        reads_backdrop: material.reads_backdrop,
        animated: material.animated,
        error: error.map(|e| e.labeled(&material.label)),
        key,
    }
}

/// FNV-1a, never zero: zero is [`MaterialId::NONE`].
fn hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.bytes() {
        h = (h ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    h.max(1)
}

impl SharedResources {
    /// Register `material` for every window, validating it now. Registering the same
    /// description again, from any window and in any frame, returns the same id without
    /// compiling or allocating anything.
    ///
    /// An invalid description is rejected with a message that points into its source. It is
    /// remembered, so asking again costs nothing and returns the same error.
    pub fn try_register_material(&self, material: &Material) -> Result<MaterialId, MaterialError> {
        let program = self.materials().register(material)?;
        match &program.error {
            Some(error) => Err(error.clone()),
            None => Ok(program.source.id),
        }
    }

    /// Like [`try_register_material`](Self::try_register_material), but an invalid description
    /// still gets an id: drawing with it paints the draw's fallback, and
    /// [`material_error`](Self::material_error) says why. A description rejected for the
    /// registry limit gets [`MaterialId::NONE`].
    pub fn register_material(&self, material: &Material) -> MaterialId {
        self.materials()
            .register(material)
            .map_or(MaterialId::NONE, |program| program.source.id)
    }

    /// Why `id` cannot be drawn: the error of a rejected registration.
    pub fn material_error(&self, id: MaterialId) -> Option<MaterialError> {
        self.materials().get(id).and_then(|p| p.error.clone())
    }

    /// Registered materials, valid or not.
    pub fn material_count(&self) -> usize {
        self.materials().len()
    }

    /// Forget a material. Draws that still use `id` fall back until it is registered again;
    /// renderers drop its pipelines when no frame uses it any more.
    pub fn remove_material(&self, id: MaterialId) -> bool {
        self.materials().remove(id)
    }
}
