//! Several renderers on one device: shared pipelines and atlas, separate buffers, bounded
//! growth and release on drop.

use super::{
    harness::{host, Host, Spec},
    scene,
};
use wgpu::TextureFormat as F;
use zaxis::{EmbedViewport, EmbeddedRenderer, SharedResources};

const SIZE: [u32; 2] = [320, 224];

fn first(host: &Host, spec: &Spec) -> EmbeddedRenderer {
    EmbeddedRenderer::new_with_adapter(
        host.device.clone(),
        host.queue.clone(),
        &host.adapter,
        spec.options(),
    )
    .unwrap()
}

#[test]
fn siblings_share_pipelines_and_the_atlas_but_not_buffers() {
    let Some(host) = host("embedded siblings") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let resources = SharedResources::new();
    let mut panels: Vec<_> = (0..3)
        .map(|n| {
            scene::controls_in(
                &mut scene::sharing(&resources, SIZE, 1.0),
                &format!("Panel {n}"),
            )
        })
        .collect();
    let mut renderers = vec![first(&host, &spec)];
    for _ in 0..3 {
        let sibling = renderers[0].create_sibling(spec.options()).unwrap();
        renderers.push(sibling);
    }
    assert_eq!(renderers[0].shared_pipeline_targets(), 1);
    renderers[0]
        .prepare(&panels[0], EmbedViewport::whole(SIZE))
        .unwrap();
    let uploaded = renderers[0].stats();
    assert!(uploaded.texture_uploads > 0 && uploaded.geometry_uploads == 1);
    for (renderer, data) in renderers[1..].iter_mut().zip(&panels[1..]) {
        renderer.prepare(data, EmbedViewport::whole(SIZE)).unwrap();
        let stats = renderer.stats();
        // The glyph atlas pages the first renderer uploaded are already there.
        assert_eq!(stats.texture_creations, 0, "{stats:?}");
        assert_eq!(stats.geometry_uploads, 1);
    }
    // Same options, same pipelines: nothing was compiled for the siblings.
    assert_eq!(renderers[0].shared_pipeline_targets(), 1);
    // Another target format adds exactly one set, and only once however many ask for it.
    let wide = Spec::new(F::Rgba16Float, SIZE);
    let a = renderers[0].create_sibling(wide.options()).unwrap();
    let b = renderers[0].create_sibling(wide.options()).unwrap();
    assert_eq!(a.shared_pipeline_targets(), 2);
    drop((a, b));
    // A frame prepared on one is untouched by preparing another.
    panels.swap(0, 1);
    renderers[1]
        .prepare(&panels[0], EmbedViewport::whole(SIZE))
        .unwrap();
    assert_eq!(renderers[0].stats().geometry_uploads, 1);
}

#[test]
fn preparing_an_unchanged_frame_uploads_nothing() {
    let Some(host) = host("embedded revisions") else {
        return;
    };
    let spec = Spec::new(F::Bgra8UnormSrgb, SIZE);
    let data = scene::controls(SIZE, 1.0);
    let mut embed = first(&host, &spec);
    for _ in 0..4 {
        embed.prepare(&data, EmbedViewport::whole(SIZE)).unwrap();
        host.in_pass(&spec, true, |pass| {
            embed.record(pass).unwrap();
        });
    }
    let stats = embed.stats();
    assert_eq!(stats.geometry_uploads, 1);
    assert_eq!(stats.texture_creations, data.textures.len() as u64);
    assert_eq!(stats.texture_reuploads, 0);
    assert_eq!(stats.presented_frames, 4);
    assert!(stats.draw_calls >= 4);
}

#[test]
fn backdrop_targets_are_released_on_request_and_on_drop() {
    let Some(host) = host("embedded release") else {
        return;
    };
    let Some(before) = host.device.generate_allocator_report() else {
        eprintln!("SKIPPED release: the backend reports no allocations");
        return;
    };
    let spec = Spec::new(F::Rgba8UnormSrgb, SIZE);
    let data = scene::blur_panel(
        SIZE,
        1.0,
        zaxis::Rect::from_min_size(zaxis::vec2(20.0, 20.0), zaxis::vec2(200.0, 150.0)),
        12.0,
    );
    let mut embed = first(&host, &spec);
    host.frame(
        &spec,
        true,
        wgpu::TextureUsages::empty(),
        |encoder, target| {
            embed
                .render_to(encoder, &target.view, None, zaxis::EmbedLoad::Keep, &data)
                .unwrap();
        },
    );
    assert!(embed.stats().blur_target_bytes > 0);
    embed.release_targets();
    assert_eq!(embed.stats().blur_target_bytes, 0);
    drop(embed);
    host.device
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    let after = host.device.generate_allocator_report().unwrap();
    assert!(
        after.total_allocated_bytes <= before.total_allocated_bytes + (4 << 20),
        "{} bytes before, {} after",
        before.total_allocated_bytes,
        after.total_allocated_bytes
    );
}
