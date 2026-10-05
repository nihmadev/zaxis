use super::{show_ui, State};
use zaxis::{PresentationMode, RenderStatus};

// Run explicitly with --smoke-test to verify the actual GPU pipeline and uploads.
pub(super) fn verify_gpu_cache(state: &mut State) {
    let cpu = state.context.cache_stats();
    let gpu = state.renderer.stats();
    state
        .context
        .run(|context| show_ui(context, &mut state.clicks, &mut state.checked));
    assert_eq!(
        state.context.cache_stats().tessellated_elements,
        cpu.tessellated_elements
    );
    assert_eq!(
        state.context.cache_stats().geometry_rebuilds,
        cpu.geometry_rebuilds
    );
    assert_eq!(
        state
            .renderer
            .render(state.context.draw_data(), state.context.style().background)
            .unwrap(),
        RenderStatus::Presented
    );
    state.context.request_repaint();
    state
        .context
        .run(|context| show_ui(context, &mut state.clicks, &mut state.checked));
    assert_eq!(
        state.context.cache_stats().tessellated_elements,
        cpu.tessellated_elements
    );
    assert_eq!(
        state.context.cache_stats().geometry_rebuilds,
        cpu.geometry_rebuilds
    );
    state.renderer.resize(state.window.inner_size()).unwrap();
    assert_eq!(
        state
            .renderer
            .render(state.context.draw_data(), state.context.style().background)
            .unwrap(),
        RenderStatus::Presented
    );
    assert_eq!(
        state.renderer.stats().geometry_uploads,
        gpu.geometry_uploads
    );
    assert_eq!(state.renderer.stats().texture_uploads, gpu.texture_uploads);
    state.checked = !state.checked;
    state
        .context
        .run(|context| show_ui(context, &mut state.clicks, &mut state.checked));
    assert_eq!(
        state.context.cache_stats().tessellated_elements,
        cpu.tessellated_elements + 1
    );
    assert_eq!(
        state
            .renderer
            .render(state.context.draw_data(), state.context.style().background)
            .unwrap(),
        RenderStatus::Presented
    );
    assert_eq!(
        state.renderer.stats().geometry_uploads,
        gpu.geometry_uploads + 1
    );
    assert_eq!(state.renderer.stats().texture_uploads, gpu.texture_uploads);
    let sparse_bytes = state.renderer.stats().geometry_upload_bytes - gpu.geometry_upload_bytes;
    let full_bytes =
        (state.context.draw_data().vertices.len() * std::mem::size_of::<zaxis::Vertex>()
            + state.context.draw_data().indices.len() * std::mem::size_of::<u32>()) as u64;
    assert!(
        sparse_bytes > 0 && sparse_bytes < full_bytes,
        "a checkbox uploaded the whole frame"
    );

    // A backend can skip UI passes. Dirty ranges cover only their named parent,
    // so presenting after two changes must safely upload complete geometry.
    for _ in 0..2 {
        state.checked = !state.checked;
        state
            .context
            .run(|context| show_ui(context, &mut state.clicks, &mut state.checked));
    }
    let before_skip = state.renderer.stats();
    assert_eq!(
        state
            .renderer
            .render(state.context.draw_data(), state.context.style().background)
            .unwrap(),
        RenderStatus::Presented
    );
    let full_bytes =
        (state.context.draw_data().vertices.len() * std::mem::size_of::<zaxis::Vertex>()
            + state.context.draw_data().indices.len() * std::mem::size_of::<u32>()) as u64;
    assert_eq!(
        state.renderer.stats().geometry_upload_bytes - before_skip.geometry_upload_bytes,
        full_bytes
    );
    let original_mode = state.renderer.presentation_mode();
    let before_switch = state.renderer.stats();
    for mode in [
        PresentationMode::Vsync,
        PresentationMode::Immediate,
        original_mode,
    ] {
        state.renderer.set_presentation_mode(mode);
        assert_eq!(state.renderer.presentation_mode(), mode);
        assert_eq!(
            state
                .renderer
                .render(state.context.draw_data(), state.context.style().background)
                .unwrap(),
            RenderStatus::Presented
        );
    }
    let after_switch = state.renderer.stats();
    assert_eq!(
        after_switch.geometry_uploads,
        before_switch.geometry_uploads
    );
    assert_eq!(after_switch.texture_uploads, before_switch.texture_uploads);
    println!(
        "GPU smoke test passed on {:?}: {:?}",
        state.renderer.adapter_info(),
        state.renderer.stats()
    );
    if std::env::args().any(|arg| arg == "--profile") {
        let mut ui_time = std::time::Duration::ZERO;
        let mut render_time = std::time::Duration::ZERO;
        let mut max_render = std::time::Duration::ZERO;
        for _ in 0..120 {
            state.checked = !state.checked;
            state.context.request_repaint();
            let start = zaxis::Instant::now();
            state
                .context
                .run(|context| show_ui(context, &mut state.clicks, &mut state.checked));
            ui_time += start.elapsed();
            let start = zaxis::Instant::now();
            assert_eq!(
                state
                    .renderer
                    .render(state.context.draw_data(), state.context.style().background)
                    .unwrap(),
                RenderStatus::Presented
            );
            let elapsed = start.elapsed();
            render_time += elapsed;
            max_render = max_render.max(elapsed);
        }
        println!(
            "120 changing frames ({original_mode:?}): UI {:.3} ms/frame, render/present {:.3} ms/frame (max {:.3} ms)",
            ui_time.as_secs_f64() * 1000.0 / 120.0,
            render_time.as_secs_f64() * 1000.0 / 120.0,
            max_render.as_secs_f64() * 1000.0,
        );
    }
}
