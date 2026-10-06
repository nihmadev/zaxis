//! `--smoke-test`: hover, pointer motion, leave, idle, scroll and resize on the real runner.

use std::time::{Duration, Instant};
use zaxis::{testing::Driver, vec2, Context, Frame, Rect, Vec2};

#[derive(Default)]
pub struct Smoke {
    phase: usize,
    waited: usize,
    idle: Option<(u64, Instant)>,
}

/// Bounds of the first card effect in the frame, in logical pixels.
fn first_card(c: &mut Context) -> Option<Rect> {
    let data = c.draw_data();
    let command = data.commands.iter().find(|c| c.material.is_some())?;
    let mut bounds: Option<Rect> = None;
    for &i in &data.indices[command.indices.start as usize..command.indices.end as usize] {
        let p = Vec2::from_array(data.vertices[i as usize].position);
        bounds = Some(bounds.map_or(Rect::from_min_max(p, p), |b| {
            Rect::from_min_max(b.min.min(p), b.max.max(p))
        }));
    }
    bounds
}

/// The parameters of the first material command: pointer (x, y) and hover.
fn first_params(c: &mut Context) -> Option<[f32; 3]> {
    let data = c.draw_data();
    let draw = data.commands.iter().find_map(|c| c.material.as_ref())?;
    let block = &data.material_uniforms[draw.uniforms.start as usize..draw.uniforms.end as usize];
    let f = |at: usize| f32::from_ne_bytes(block[at..at + 4].try_into().unwrap());
    // 32 bytes of frame data, then pointer (vec2) and hover (f32).
    Some([f(32), f(36), f(40)])
}

impl Smoke {
    fn waiting(&mut self, ready: bool) -> bool {
        self.waited += 1;
        assert!(
            self.waited < 900,
            "smoke test stalled in phase {}",
            self.phase
        );
        if ready {
            self.phase += 1;
            self.waited = 0;
        }
        ready
    }

    pub fn step(&mut self, c: &mut Context, frame: &mut Frame<'_>) {
        if !matches!(self.phase, 4 | 5) {
            c.request_repaint();
        }
        let card = first_card(c);
        match self.phase {
            // Cards painted, all through one material.
            0 => {
                if self.waiting(card.is_some() && c.draw_data().textures.len() > 1) {
                    let data = c.draw_data();
                    assert_eq!(data.materials.len(), 1, "one material for every card");
                    let at_rest = first_params(c).unwrap();
                    assert_eq!(at_rest[2], 0.0, "no hover at rest");
                }
            }
            // The pointer enters a card: hover settles at 1 with the pointer near the left edge.
            1 => {
                let card = card.unwrap();
                c.move_pointer(card.min + card.size() * vec2(0.15, 0.5));
                let params = first_params(c).unwrap();
                if self.waiting(params[2] > 0.99 && params[0] < 0.25) {
                    println!(
                        "hover settled: pointer ({:.2}, {:.2})",
                        params[0], params[1]
                    );
                }
            }
            // It moves to the other side: the shift follows.
            2 => {
                let card = card.unwrap();
                c.move_pointer(card.min + card.size() * vec2(0.85, 0.5));
                let params = first_params(c).unwrap();
                self.waiting(params[0] > 0.75);
            }
            // It leaves: back to rest.
            3 => {
                c.move_pointer(vec2(2.0, 2.0));
                let params = first_params(c).unwrap();
                if self.waiting(
                    params[2] < 0.01
                        && (params[0] - 0.5).abs() < 0.01
                        && !c.wants_animation_frame(),
                ) {
                    // From here only the wake-up below may ask for a frame.
                    self.idle = Some((frame.stats().frames_presented, Instant::now()));
                    c.request_repaint_after(Duration::from_millis(900));
                }
            }
            // Idle: nothing but the wake-up asks for a frame.
            4 => {
                let (start, at) = self.idle.expect("idle starts when the pointer has left");
                if at.elapsed() > Duration::from_millis(800) {
                    let drawn = frame.stats().frames_presented - start;
                    assert!(drawn <= 2, "{drawn} frames were presented while idle");
                    println!("idle: {drawn} frames in {:?}", at.elapsed());
                    self.phase = 5;
                    c.request_repaint();
                } else {
                    c.request_repaint_after(Duration::from_millis(900) - at.elapsed());
                }
            }
            // Scroll the grid and resize the window.
            5 => {
                c.request_repaint();
                c.scroll_wheel(vec2(0.0, -300.0));
                let _ = frame
                    .window()
                    .request_inner_size(zaxis::winit::dpi::LogicalSize::new(760.0, 600.0));
                self.waiting(true);
            }
            6 => {
                let resized = c.viewport().max.x < 800.0;
                self.waiting(resized);
                if resized {
                    assert!(first_card(c).is_some(), "the cards survive a resize");
                    println!("pixel cards smoke passed");
                    frame.close();
                }
            }
            _ => {}
        }
    }
}
