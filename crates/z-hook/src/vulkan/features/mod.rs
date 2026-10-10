//! Adding what wgpu needs to the host's `VkDeviceCreateInfo`.
//!
//! wgpu opens a device from raw handles only if it was created with the extensions and
//! features wgpu would have asked for. The host asked for its own, so the layer merges: it
//! adds extension names, ORs feature booleans into the host's structures where they exist
//! (the booleans are only ever raised), and links copies of wgpu's structures in front of the
//! chain where the host has none. Structures wgpu sets nothing in are left out. A feature
//! that a host's `VkPhysicalDeviceVulkan1xFeatures` already covers is set there, because a
//! chain must not hold both an aggregate and an individual structure for the same feature.

use ash::vk::{self, TaggedStructure};
use std::{
    ffi::{c_char, c_void, CStr},
    mem::size_of,
    ptr,
};

/// The structure header: `sType` and `pNext`, 16 bytes; everything after it in a feature
/// structure is a `VkBool32`.
const HEADER: usize = size_of::<vk::BaseOutStructure<'static>>();

macro_rules! feature_sizes {
    ($($t:ty),* $(,)?) => {
        /// The size of a feature structure the merge can copy, by `sType`.
        fn feature_size(s_type: vk::StructureType) -> Option<usize> {
            $(if s_type == <$t as TaggedStructure>::STRUCTURE_TYPE { return Some(size_of::<$t>()); })*
            None
        }
    };
}

feature_sizes!(
    vk::PhysicalDeviceFeatures2<'static>,
    vk::PhysicalDeviceVulkan11Features<'static>,
    vk::PhysicalDeviceVulkan12Features<'static>,
    vk::PhysicalDeviceVulkan13Features<'static>,
    vk::PhysicalDeviceDescriptorIndexingFeatures<'static>,
    vk::PhysicalDeviceTimelineSemaphoreFeatures<'static>,
    vk::PhysicalDeviceImageRobustnessFeatures<'static>,
    vk::PhysicalDeviceRobustness2FeaturesEXT<'static>,
    vk::PhysicalDeviceMultiviewFeatures<'static>,
    vk::PhysicalDeviceSamplerYcbcrConversionFeatures<'static>,
    vk::PhysicalDeviceTextureCompressionASTCHDRFeatures<'static>,
    vk::PhysicalDeviceShaderFloat16Int8Features<'static>,
    vk::PhysicalDevice16BitStorageFeatures<'static>,
    vk::PhysicalDeviceZeroInitializeWorkgroupMemoryFeatures<'static>,
    vk::PhysicalDeviceShaderAtomicInt64Features<'static>,
    vk::PhysicalDeviceSubgroupSizeControlFeatures<'static>,
    vk::PhysicalDeviceMaintenance4Features<'static>,
    vk::PhysicalDeviceShaderIntegerDotProductFeatures<'static>,
    vk::PhysicalDeviceVulkanMemoryModelFeatures<'static>,
    vk::PhysicalDeviceShaderDrawParametersFeatures<'static>,
    vk::PhysicalDeviceBufferDeviceAddressFeatures<'static>,
    vk::PhysicalDeviceShaderImageAtomicInt64FeaturesEXT<'static>,
    vk::PhysicalDeviceShaderAtomicFloatFeaturesEXT<'static>,
    vk::PhysicalDeviceFragmentShaderBarycentricFeaturesKHR<'static>,
    vk::PhysicalDevicePortabilitySubsetFeaturesKHR<'static>,
    vk::PhysicalDeviceCooperativeMatrixFeaturesKHR<'static>,
    vk::PhysicalDeviceAccelerationStructureFeaturesKHR<'static>,
    vk::PhysicalDeviceRayQueryFeaturesKHR<'static>,
    vk::PhysicalDeviceRayTracingPipelineFeaturesKHR<'static>,
    vk::PhysicalDeviceRayTracingPositionFetchFeaturesKHR<'static>,
    vk::PhysicalDeviceMeshShaderFeaturesEXT<'static>,
);

type Node = vk::BaseOutStructure<'static>;

/// The booleans of a feature structure: everything after its header.
fn words(node: *const Node) -> *mut u32 {
    node.cast::<u8>().wrapping_add(HEADER).cast_mut().cast()
}

fn bool_count(size: usize) -> usize {
    (size - HEADER) / size_of::<u32>()
}

/// Whether any feature boolean after the header is set.
///
/// # Safety
/// `node` must point to a feature structure of `size` bytes.
unsafe fn any_set(node: *const Node, size: usize) -> bool {
    // SAFETY: `size` bytes follow the pointer; the part after the header is VkBool32s.
    (0..bool_count(size)).any(|i| unsafe { *words(node).add(i) } != 0)
}

/// OR `count` booleans of `from` into `into`.
///
/// # Safety
/// Both must point to `count` readable (and, for `into`, writable) `VkBool32`s.
unsafe fn or_bools(into: *mut u32, from: *const u32, count: usize) {
    for i in 0..count {
        // SAFETY: the caller guarantees `count` booleans on both sides.
        unsafe { *into.add(i) |= *from.add(i) };
    }
}

/// OR the booleans of `from` into `into`, same `sType` and size.
///
/// # Safety
/// Both must point to feature structures of `size` bytes.
unsafe fn or_into(into: *mut Node, from: *const Node, size: usize) {
    // SAFETY: both structures span `size` bytes.
    unsafe { or_bools(words(into), words(from), bool_count(size)) };
}

/// Find the first node of `s_type` in a chain.
///
/// # Safety
/// `head` must start a valid `pNext` chain.
unsafe fn find(head: *const c_void, s_type: vk::StructureType) -> *mut Node {
    let mut node = head.cast::<Node>();
    while !node.is_null() {
        // SAFETY: every node of a pNext chain starts with sType and pNext.
        unsafe {
            if (*node).s_type == s_type {
                return node.cast_mut();
            }
            node = (*node).p_next;
        }
    }
    ptr::null_mut()
}

/// The device create info the host asked for, with wgpu's needs added.
pub(super) struct Merged {
    info: vk::DeviceCreateInfo<'static>,
    // Everything `info` points to that is not the host's: kept alive with it.
    _core: Box<vk::PhysicalDeviceFeatures>,
    _nodes: Vec<Box<[u64]>>,
    _names: Vec<*const c_char>,
}

impl Merged {
    pub fn info(&self) -> &vk::DeviceCreateInfo<'static> {
        &self.info
    }
}

// SAFETY: the raw pointers point at the host's data and at `Merged`'s own boxes, and are only
// read during one `vkCreateDevice` call on the thread that built them.
unsafe impl Send for Merged {}

/// Why the merge was refused: the host's device stays as it asked for it.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Refused(pub String);

/// Merge `wanted` (what `add_to_device_create` produced) and `extensions` into `host`.
///
/// Host structures are modified in place where they hold a feature wgpu needs (raised, never
/// lowered); the host does not read them again, and a repeat is idempotent.
///
/// # Safety
/// `host` and `wanted` must be valid create infos with valid `pNext` chains, and the host's
/// extension names valid C strings.
pub(super) unsafe fn merge(
    host: &vk::DeviceCreateInfo<'_>,
    wanted: &vk::DeviceCreateInfo<'_>,
    extensions: &[&'static CStr],
) -> Result<Merged, Refused> {
    // SAFETY: the lifetime is erased; `Merged` and the host's data outlive the create call.
    let mut info: vk::DeviceCreateInfo<'static> = unsafe { std::mem::transmute(*host) };
    let mut head: *const c_void = host.p_next;
    let mut nodes: Vec<Box<[u64]>> = Vec::new();

    // Core features: in a host's `VkPhysicalDeviceFeatures2` if it has one, else `pEnabledFeatures`.
    let mut core = Box::new(vk::PhysicalDeviceFeatures::default());
    // SAFETY: the host's chain is valid.
    let features2 = unsafe { find(head, vk::StructureType::PHYSICAL_DEVICE_FEATURES_2) };
    const CORE_BOOLS: usize = size_of::<vk::PhysicalDeviceFeatures>() / size_of::<u32>();
    // SAFETY: the host's pEnabledFeatures is null or valid; wgpu's is valid; `Features2` is a
    // valid structure whose `features` follow its header.
    unsafe {
        let wanted_core = wanted
            .p_enabled_features
            .as_ref()
            .copied()
            .unwrap_or_default();
        let from = ptr::from_ref(&wanted_core).cast::<u32>();
        if features2.is_null() {
            if let Some(host_core) = host.p_enabled_features.as_ref() {
                *core = *host_core;
            }
            or_bools(ptr::from_mut(&mut *core).cast(), from, CORE_BOOLS);
            info.p_enabled_features = &*core;
        } else {
            or_bools(words(features2), from, CORE_BOOLS);
        }
    }

    // wgpu's feature structures.
    // SAFETY: wgpu's chain is valid.
    let mut node = wanted.p_next.cast::<Node>();
    while !node.is_null() {
        // SAFETY: every node of a pNext chain starts with sType and pNext.
        let (s_type, next) = unsafe { ((*node).s_type, (*node).p_next) };
        let Some(size) = feature_size(s_type) else {
            return Err(Refused(format!(
                "wgpu asks for a structure the layer cannot merge: {s_type:?}"
            )));
        };
        // SAFETY: `node` is a feature structure of `size` bytes.
        if unsafe { any_set(node, size) } {
            // SAFETY: the host's chain is valid.
            let same = unsafe { find(head, s_type) };
            if !same.is_null() {
                // SAFETY: same sType, so same size.
                unsafe { or_into(same, node, size) };
            } else if !unsafe { fold_into_aggregate(head, node, s_type) } {
                let mut copy = vec![0u64; size.div_ceil(8)].into_boxed_slice();
                // SAFETY: `copy` spans at least `size` bytes; `node` spans `size`.
                unsafe {
                    ptr::copy_nonoverlapping(
                        node.cast::<u8>(),
                        copy.as_mut_ptr().cast::<u8>(),
                        size,
                    );
                    let new = copy.as_mut_ptr().cast::<Node>();
                    (*new).p_next = head.cast_mut().cast();
                    head = new.cast();
                }
                nodes.push(copy);
            }
        }
        node = next;
    }
    info.p_next = head;

    // Extension names.
    // SAFETY: the host's names are valid C strings.
    let host_names: Vec<&CStr> = (0..host.enabled_extension_count as usize)
        .map(|i| unsafe { CStr::from_ptr(*host.pp_enabled_extension_names.add(i)) })
        .collect();
    let mut names: Vec<*const c_char> = host_names.iter().map(|n| n.as_ptr()).collect();
    for extension in extensions {
        if !host_names.contains(extension) {
            names.push(extension.as_ptr());
        }
    }
    info.enabled_extension_count = names.len() as u32;
    info.pp_enabled_extension_names = names.as_ptr();
    Ok(Merged {
        info,
        _core: core,
        _nodes: nodes,
        _names: names,
    })
}

/// Set the booleans of `node` (an individual feature structure) in the host's aggregate that
/// covers them. `false` when the host has no such aggregate, or none that covers this
/// structure; the caller then chains the structure itself.
///
/// # Safety
/// `head` must start a valid chain and `node` be a structure of type `s_type`.
unsafe fn fold_into_aggregate(
    head: *const c_void,
    node: *const Node,
    s_type: vk::StructureType,
) -> bool {
    use vk::StructureType as S;
    // SAFETY: the chain is valid.
    let (v11, v12, v13) = unsafe {
        (
            find(head, S::PHYSICAL_DEVICE_VULKAN_1_1_FEATURES)
                .cast::<vk::PhysicalDeviceVulkan11Features<'static>>(),
            find(head, S::PHYSICAL_DEVICE_VULKAN_1_2_FEATURES)
                .cast::<vk::PhysicalDeviceVulkan12Features<'static>>(),
            find(head, S::PHYSICAL_DEVICE_VULKAN_1_3_FEATURES)
                .cast::<vk::PhysicalDeviceVulkan13Features<'static>>(),
        )
    };
    macro_rules! fold {
        ($agg:ident, $ty:ty, { $($field:ident),* }) => {{
            if $agg.is_null() { return false; }
            // SAFETY: `node` has the type `s_type` names.
            let from = unsafe { &*node.cast::<$ty>() };
            // SAFETY: the aggregate is the host's valid structure.
            unsafe { $( (*$agg).$field |= from.$field; )* }
            return true;
        }};
    }
    if s_type == S::PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES {
        fold!(v12, vk::PhysicalDeviceTimelineSemaphoreFeatures<'static>, {
            timeline_semaphore
        });
    }
    if s_type == S::PHYSICAL_DEVICE_SHADER_FLOAT16_INT8_FEATURES {
        fold!(v12, vk::PhysicalDeviceShaderFloat16Int8Features<'static>, { shader_float16, shader_int8 });
    }
    if s_type == S::PHYSICAL_DEVICE_IMAGE_ROBUSTNESS_FEATURES {
        fold!(v13, vk::PhysicalDeviceImageRobustnessFeatures<'static>, {
            robust_image_access
        });
    }
    if s_type == S::PHYSICAL_DEVICE_ZERO_INITIALIZE_WORKGROUP_MEMORY_FEATURES {
        fold!(
            v13,
            vk::PhysicalDeviceZeroInitializeWorkgroupMemoryFeatures<'static>,
            { shader_zero_initialize_workgroup_memory }
        );
    }
    if s_type == S::PHYSICAL_DEVICE_SUBGROUP_SIZE_CONTROL_FEATURES {
        fold!(v13, vk::PhysicalDeviceSubgroupSizeControlFeatures<'static>, { subgroup_size_control, compute_full_subgroups });
    }
    if s_type == S::PHYSICAL_DEVICE_SHADER_INTEGER_DOT_PRODUCT_FEATURES {
        fold!(
            v13,
            vk::PhysicalDeviceShaderIntegerDotProductFeatures<'static>,
            { shader_integer_dot_product }
        );
    }
    if s_type == S::PHYSICAL_DEVICE_MAINTENANCE_4_FEATURES {
        fold!(v13, vk::PhysicalDeviceMaintenance4Features<'static>, {
            maintenance4
        });
    }
    if s_type == S::PHYSICAL_DEVICE_MULTIVIEW_FEATURES {
        fold!(v11, vk::PhysicalDeviceMultiviewFeatures<'static>, { multiview, multiview_geometry_shader, multiview_tessellation_shader });
    }
    if s_type == S::PHYSICAL_DEVICE_SHADER_DRAW_PARAMETERS_FEATURES {
        fold!(
            v11,
            vk::PhysicalDeviceShaderDrawParametersFeatures<'static>,
            { shader_draw_parameters }
        );
    }
    if s_type == S::PHYSICAL_DEVICE_VULKAN_MEMORY_MODEL_FEATURES {
        fold!(v12, vk::PhysicalDeviceVulkanMemoryModelFeatures<'static>, { vulkan_memory_model, vulkan_memory_model_device_scope, vulkan_memory_model_availability_visibility_chains });
    }
    if s_type == S::PHYSICAL_DEVICE_SHADER_ATOMIC_INT64_FEATURES {
        fold!(v12, vk::PhysicalDeviceShaderAtomicInt64Features<'static>, { shader_buffer_int64_atomics, shader_shared_int64_atomics });
    }
    if s_type == S::PHYSICAL_DEVICE_BUFFER_DEVICE_ADDRESS_FEATURES {
        fold!(v12, vk::PhysicalDeviceBufferDeviceAddressFeatures<'static>, { buffer_device_address, buffer_device_address_capture_replay, buffer_device_address_multi_device });
    }
    // Everything else is an extension structure no aggregate covers.
    false
}

#[cfg(test)]
mod tests;
