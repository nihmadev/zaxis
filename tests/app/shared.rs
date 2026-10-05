//! Several windows are several contexts: strictly separate input and retained state,
//! one glyph atlas, one image cache, one theme. No GPU or native window is involved.

use crate::{
    Context, Image, ImageSource, Modal, Rect, Root, SharedResources, TextEdit, TextureId, Theme,
    Vec2,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{DeviceId, ElementState, Ime, MouseButton, WindowEvent},
};

fn window(resources: &SharedResources) -> Context {
    let mut context = Context::with_shared(resources);
    context.set_viewport(PhysicalSize::new(600, 400), 1.0);
    context
}

fn field(context: &mut Context, text: &mut String) -> (Rect, bool) {
    let mut out = (Rect::default(), false);
    context.run(|context| {
        Root::new().show(context, |ui| {
            // The same Id in every window, on purpose.
            let response = ui.add(TextEdit::new(text).id_source("name").width(200.0));
            out = (response.rect, response.has_focus);
        });
    });
    out
}

fn move_pointer(context: &mut Context, at: Vec2) {
    context.on_window_event(&WindowEvent::CursorMoved {
        device_id: DeviceId::dummy(),
        position: PhysicalPosition::new(f64::from(at.x), f64::from(at.y)),
    });
}

fn button(context: &mut Context, state: ElementState) {
    context.on_window_event(&WindowEvent::MouseInput {
        device_id: DeviceId::dummy(),
        state,
        button: MouseButton::Left,
    });
}

fn click(context: &mut Context, at: Vec2) {
    move_pointer(context, at);
    button(context, ElementState::Pressed);
    button(context, ElementState::Released);
}

fn preedit(context: &mut Context, text: &str) {
    context.on_window_event(&WindowEvent::Ime(Ime::Preedit(text.into(), Some((0, 0)))));
}

fn focus(context: &mut Context, focused: bool) {
    context.on_window_event(&WindowEvent::Focused(focused));
}

#[test]
fn the_same_id_in_two_windows_has_independent_focus_and_text() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    let (mut text_a, mut text_b) = (String::new(), String::from("untouched"));
    let (rect, _) = field(&mut a, &mut text_a);
    field(&mut b, &mut text_b);
    click(&mut a, rect.center());
    assert!(field(&mut a, &mut text_a).1, "focused in A");
    a.on_text_event("hi");
    field(&mut a, &mut text_a);
    assert_eq!(text_a, "hi");
    let (_, focused_b) = field(&mut b, &mut text_b);
    assert!(!focused_b, "same Id, other window: not focused");
    assert_eq!(text_b, "untouched");
    // Typing into B's context reaches B only.
    click(&mut b, rect.center());
    field(&mut b, &mut text_b);
    b.on_text_event("yo");
    field(&mut b, &mut text_b);
    assert_eq!(text_b, "untouchedyo");
    assert_eq!(text_a, "hi");
}

#[test]
fn pointer_modifiers_and_hover_belong_to_one_window() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    move_pointer(&mut a, Vec2::new(40.0, 40.0));
    button(&mut a, ElementState::Pressed);
    assert!(a.input().primary_down);
    assert!(b.input().pointer.is_none() && !b.input().primary_down);
}

#[test]
fn losing_focus_resets_capture_buttons_and_ime_of_that_window_only() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    let (mut text_a, mut text_b) = (String::new(), String::new());
    let (rect_a, _) = field(&mut a, &mut text_a);
    let (rect_b, _) = field(&mut b, &mut text_b);
    // Both windows have a focused field mid-composition and a held button.
    for (context, rect, text) in [(&mut a, rect_a, &mut text_a), (&mut b, rect_b, &mut text_b)] {
        click(context, rect.center());
        field(context, text);
        preedit(context, "にほん");
        button(context, ElementState::Pressed);
        field(context, text);
        assert!(context.ime_cursor_area().is_some());
        assert!(context.input().primary_down);
    }
    focus(&mut a, false);
    field(&mut a, &mut text_a);
    assert!(!a.input().primary_down, "the held button is released");
    assert!(a.input().pointer.is_none(), "hover is cleared");
    assert!(
        a.ime_cursor_area().is_none(),
        "no IME area without a focused field"
    );
    // B was never told anything: its composition, button and pointer are intact.
    field(&mut b, &mut text_b);
    assert!(b.input().primary_down && b.input().pointer.is_some());
    assert!(b.ime_cursor_area().is_some(), "B keeps its own IME area");
    // A text commit while A is unfocused has no field to land in.
    a.on_window_event(&WindowEvent::Ime(Ime::Commit("x".into())));
    field(&mut a, &mut text_a);
    assert!(text_a.is_empty());
}

#[test]
fn a_modal_blocks_only_its_own_window() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    let mut open = true;
    for _ in 0..3 {
        a.run(|context| {
            Root::new().show(context, |ui| {
                Modal::new("confirm").show(ui, &mut open, |ui| {
                    ui.label("Discard changes?");
                });
            });
        });
        b.run(|context| {
            Root::new().show(context, |ui| ui.label("untouched"));
        });
    }
    assert!(open);
    move_pointer(&mut a, Vec2::new(30.0, 30.0));
    move_pointer(&mut b, Vec2::new(30.0, 30.0));
    assert!(
        a.input().pointer.is_none(),
        "input outside the modal reads idle in A"
    );
    assert!(b.input().pointer.is_some(), "B is unaffected");
}

#[test]
fn one_theme_call_reaches_every_window_in_its_next_frame() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    for context in [&mut a, &mut b] {
        context.run(|_| {});
        assert_eq!(context.style(), &Theme::dark().resolve());
    }
    resources.set_theme(Theme::light());
    // The call itself changes nothing yet and asks both windows to repaint.
    assert_eq!(a.style(), &Theme::dark().resolve());
    let now = Instant::now();
    assert!(a.needs_repaint_at(now) && b.needs_repaint_at(now));
    a.run(|_| {});
    assert_eq!(a.style(), &Theme::light().resolve());
    assert_eq!(
        b.style(),
        &Theme::dark().resolve(),
        "B applies it in its own next frame"
    );
    b.run(|_| {});
    assert_eq!(b.style(), &Theme::light().resolve());
    assert!(
        !a.needs_repaint_at(Instant::now()),
        "applied once, not repeatedly"
    );
    // Windows opened later start with the current theme.
    let mut late = window(&resources);
    late.run(|_| {});
    assert_eq!(late.style(), &Theme::light().resolve());
}

fn draw_text(context: &mut Context, text: &str) {
    context.run(|context| {
        Root::new().show(context, |ui| {
            ui.label(text);
        })
    });
}

#[test]
fn windows_share_one_glyph_atlas() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    draw_text(&mut a, "Shared atlas 123");
    let pages = |context: &Context| -> Vec<(TextureId, u64)> {
        context
            .draw_data()
            .textures
            .iter()
            .map(|t| (t.id, t.revision))
            .collect()
    };
    let after_a = pages(&a);
    assert!(!after_a.is_empty());
    draw_text(&mut b, "Shared atlas 123");
    assert_eq!(
        pages(&b),
        after_a,
        "same glyphs: same pages, no new rasterization"
    );
    draw_text(&mut b, "Only B draws these: QWXZ");
    let after_b = pages(&b);
    assert_eq!(after_b.len(), after_a.len());
    assert_ne!(
        after_b, after_a,
        "new glyphs advance the shared page revision"
    );
    // A's next frame sees B's glyphs for free.
    draw_text(&mut a, "Only B draws these: QWXZ");
    assert_eq!(pages(&a), after_b);
    // A separate resource set has an atlas of its own, so IDs cannot be confused with ours.
    let mut alone = Context::new();
    alone.set_viewport(PhysicalSize::new(600, 400), 1.0);
    draw_text(&mut alone, "Shared atlas 123");
    assert!(!alone.shared_resources().same_as(&resources));
}

#[test]
fn an_image_is_decoded_once_for_all_windows() {
    let resources = SharedResources::new();
    let (mut a, mut b) = (window(&resources), window(&resources));
    let source = ImageSource::rgba([2, 2], Arc::new(vec![255; 16]));
    let show = |context: &mut Context| {
        context.run(|context| {
            Root::new().show(context, |ui| {
                ui.add(Image::new(&source).size(Vec2::new(32.0, 32.0)));
            })
        });
    };
    show(&mut a);
    let after_a = resources.image_metrics();
    assert_eq!(after_a.cache_misses, 1);
    show(&mut b);
    let after_b = resources.image_metrics();
    assert_eq!(
        after_b.cache_misses, 1,
        "the second window finds the entry A created"
    );
    assert!(after_b.cache_hits > after_a.cache_hits);
    // Both windows draw the same texture ID, so the GPU holds one texture for it.
    let image_ids = |context: &Context| -> Vec<TextureId> {
        context
            .draw_data()
            .texture_options
            .keys()
            .copied()
            .collect()
    };
    assert_eq!(image_ids(&a), image_ids(&b));
    assert!(!image_ids(&a).is_empty());
    // A handle taken in one window is valid in the other.
    let handle = a.load_image(source.clone()).unwrap();
    assert!(b.update_image(handle, source).is_ok());
}

#[test]
fn only_the_window_that_shows_an_image_wakes_for_it() {
    let resources = SharedResources::new();
    let (mut with_image, mut without) = (window(&resources), window(&resources));
    with_image.run(|context| {
        Root::new().show(context, |ui| {
            ui.add(Image::new(ImageSource::rgba([1, 1], vec![255; 4])).size(Vec2::splat(8.0)));
        })
    });
    without.run(|_| {});
    // Wait for the worker so a result is pending in the shared cache.
    let deadline = Instant::now() + Duration::from_secs(5);
    while !with_image.needs_repaint_at(Instant::now()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        with_image.needs_repaint_at(Instant::now()),
        "the decoded image needs a frame"
    );
    assert!(
        !without.needs_repaint_at(Instant::now()),
        "a window with no images is not woken"
    );
}

#[test]
fn an_animation_in_one_window_does_not_repaint_the_others() {
    let resources = SharedResources::new();
    let (mut busy, mut idle) = (window(&resources), window(&resources));
    for context in [&mut busy, &mut idle] {
        for _ in 0..3 {
            context.run(|_| {});
        }
    }
    let now = Instant::now();
    assert!(!busy.needs_repaint_at(now) && !idle.needs_repaint_at(now));
    busy.request_repaint_after(Duration::from_millis(10));
    let later = now + Duration::from_millis(11);
    assert!(busy.needs_repaint_at(later));
    assert!(
        !idle.needs_repaint_at(later),
        "the idle window has no deadline"
    );
    assert_eq!(idle.next_repaint(), None);
    assert!(busy.next_repaint().is_some());
}

#[test]
fn a_reopened_window_starts_with_clean_retained_state_and_the_old_context_is_released() {
    let resources = SharedResources::new();
    let mut first = window(&resources);
    let mut text = String::new();
    let (rect, _) = field(&mut first, &mut text);
    click(&mut first, rect.center());
    assert!(field(&mut first, &mut text).1);
    drop(first);
    // Same key, same Id, new context: nothing is focused, nothing carried over.
    let mut second = window(&resources);
    assert!(!field(&mut second, &mut text).1);
    assert!(second.input().pointer.is_none());
    assert_eq!(second.style(), &Theme::dark().resolve());
}

#[test]
fn contexts_created_without_shared_resources_are_unchanged() {
    let mut single = Context::new();
    single.set_viewport(PhysicalSize::new(300, 200), 1.0);
    single.run(|context| {
        Root::new().show(context, |ui| ui.label("one window"));
    });
    assert!(single.cache_stats().ui_passes == 1);
    let again = single.shared_resources().clone();
    assert!(again.same_as(single.shared_resources()));
}
