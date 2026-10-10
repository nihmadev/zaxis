//! Merging wgpu's needs into the host's device create info.

use super::*;

const KHR_SWAPCHAIN: &CStr = c"VK_KHR_swapchain";
const EXT_ROBUSTNESS2: &CStr = c"VK_EXT_robustness2";

fn wanted_with<'a>(
    core: &'a vk::PhysicalDeviceFeatures,
    timeline: &'a mut vk::PhysicalDeviceTimelineSemaphoreFeatures<'a>,
) -> vk::DeviceCreateInfo<'a> {
    vk::DeviceCreateInfo::default()
        .enabled_features(core)
        .push_next(timeline)
}

fn core_with_sample_rate() -> vk::PhysicalDeviceFeatures {
    vk::PhysicalDeviceFeatures::default()
        .sample_rate_shading(true)
        .robust_buffer_access(true)
}

#[test]
fn a_host_with_pointer_features_gets_them_raised_in_a_copy() {
    let host_core = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);
    let host_names = [KHR_SWAPCHAIN.as_ptr()];
    let host = vk::DeviceCreateInfo::default()
        .enabled_features(&host_core)
        .enabled_extension_names(&host_names);
    let wanted_core = core_with_sample_rate();
    let mut timeline =
        vk::PhysicalDeviceTimelineSemaphoreFeatures::default().timeline_semaphore(true);
    let wanted = wanted_with(&wanted_core, &mut timeline);
    let merged = unsafe { merge(&host, &wanted, &[KHR_SWAPCHAIN, EXT_ROBUSTNESS2]) }.unwrap();
    let features = unsafe { &*merged.info().p_enabled_features };
    assert_eq!(
        (
            features.sampler_anisotropy,
            features.sample_rate_shading,
            features.robust_buffer_access
        ),
        (1, 1, 1)
    );
    assert_eq!(
        host_core.sample_rate_shading, 0,
        "the host's own structure is not touched"
    );
    assert_eq!(
        merged.info().enabled_extension_count,
        2,
        "swapchain is not repeated"
    );
    let found = unsafe {
        find(
            merged.info().p_next,
            vk::StructureType::PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES,
        )
    };
    assert!(
        !found.is_null()
            && unsafe {
                (*found.cast::<vk::PhysicalDeviceTimelineSemaphoreFeatures>()).timeline_semaphore
            } == 1
    );
}

#[test]
fn features2_in_the_host_chain_is_raised_in_place() {
    let mut host_features = vk::PhysicalDeviceFeatures2::default();
    host_features.features.sampler_anisotropy = 1;
    let host = vk::DeviceCreateInfo::default().push_next(&mut host_features);
    let wanted_core = core_with_sample_rate();
    let mut timeline =
        vk::PhysicalDeviceTimelineSemaphoreFeatures::default().timeline_semaphore(true);
    let wanted = wanted_with(&wanted_core, &mut timeline);
    let merged = unsafe { merge(&host, &wanted, &[]) }.unwrap();
    assert!(
        merged.info().p_enabled_features.is_null(),
        "pEnabledFeatures stays unused next to Features2"
    );
    assert_eq!(host_features.features.sample_rate_shading, 1);
    assert_eq!(host_features.features.sampler_anisotropy, 1);
}

#[test]
fn a_host_aggregate_takes_the_feature_instead_of_a_second_structure() {
    let mut v12 = vk::PhysicalDeviceVulkan12Features::default().buffer_device_address(true);
    let host = vk::DeviceCreateInfo::default().push_next(&mut v12);
    let wanted_core = core_with_sample_rate();
    let mut timeline =
        vk::PhysicalDeviceTimelineSemaphoreFeatures::default().timeline_semaphore(true);
    let wanted = wanted_with(&wanted_core, &mut timeline);
    let merged = unsafe { merge(&host, &wanted, &[]) }.unwrap();
    assert_eq!(v12.timeline_semaphore, 1);
    assert_eq!(v12.buffer_device_address, 1, "the host's own feature stays");
    let individual = unsafe {
        find(
            merged.info().p_next,
            vk::StructureType::PHYSICAL_DEVICE_TIMELINE_SEMAPHORE_FEATURES,
        )
    };
    assert!(
        individual.is_null(),
        "an aggregate and an individual structure must not both be chained"
    );
}

#[test]
fn a_structure_the_host_already_chains_is_raised_not_duplicated() {
    let mut host_timeline = vk::PhysicalDeviceTimelineSemaphoreFeatures::default();
    let host = vk::DeviceCreateInfo::default().push_next(&mut host_timeline);
    let wanted_core = core_with_sample_rate();
    let mut timeline =
        vk::PhysicalDeviceTimelineSemaphoreFeatures::default().timeline_semaphore(true);
    let wanted = wanted_with(&wanted_core, &mut timeline);
    let merged = unsafe { merge(&host, &wanted, &[]) }.unwrap();
    assert_eq!(host_timeline.timeline_semaphore, 1);
    assert!(ptr::eq(
        merged.info().p_next,
        ptr::from_ref(&host_timeline).cast()
    ));
}

#[test]
fn structures_wgpu_leaves_all_false_are_not_chained() {
    let host = vk::DeviceCreateInfo::default();
    let wanted_core = core_with_sample_rate();
    let mut timeline = vk::PhysicalDeviceTimelineSemaphoreFeatures::default();
    let wanted = wanted_with(&wanted_core, &mut timeline);
    let merged = unsafe { merge(&host, &wanted, &[]) }.unwrap();
    assert!(merged.info().p_next.is_null());
}

#[test]
fn an_unknown_wgpu_structure_refuses_the_merge() {
    let host = vk::DeviceCreateInfo::default();
    let mut unknown = vk::PhysicalDeviceProperties2::default();
    let wanted = vk::DeviceCreateInfo {
        p_next: ptr::from_mut(&mut unknown).cast(),
        ..Default::default()
    };
    assert!(unsafe { merge(&host, &wanted, &[]) }.is_err());
}
