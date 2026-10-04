//! Modal input blocking, focus, close reasons, layers and lifecycle.
use super::*;
use crate::{
    vec2, Button, CloseReason, ComboBox, ComboBoxOption, Modal, Rect, Root, ScrollArea, Slider,
    TextEdit,
};
use std::time::Instant;
use winit::{dpi::PhysicalSize, event::ElementState, keyboard::KeyCode};

pub(super) struct Scene {
    pub open: bool,
    pub nested: bool,
    pub escape: bool,
    pub overlay: bool,
    pub show_under: bool,
    pub long_body: bool,
    pub under_clicks: u32,
    pub inner_clicks: u32,
    pub nested_clicks: u32,
    pub shortcut: u32,
    pub default_actions: u32,
    pub closed: Vec<CloseReason>,
    pub nested_closed: Vec<CloseReason>,
    pub under_text: String,
    pub inner_text: String,
    pub slider: f32,
    pub combo: Option<usize>,
    pub under_combo: Option<usize>,
    pub under_combo_rect: Rect,
    pub scroll_offset: f32,
    pub under_hovered: bool,
    pub under: Rect,
    pub inner: Rect,
    pub slider_rect: Rect,
    pub text_rect: Rect,
    pub inner_text_rect: Rect,
    pub combo_rect: Rect,
    pub surface: Rect,
    pub scroll_rect: Rect,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            open: false,
            nested: false,
            escape: true,
            overlay: true,
            show_under: true,
            long_body: false,
            under_clicks: 0,
            inner_clicks: 0,
            nested_clicks: 0,
            shortcut: 0,
            default_actions: 0,
            closed: Vec::new(),
            nested_closed: Vec::new(),
            under_text: String::new(),
            inner_text: String::new(),
            slider: 10.0,
            combo: None,
            under_combo: None,
            under_combo_rect: Rect::default(),
            scroll_offset: 0.0,
            under_hovered: false,
            under: Rect::default(),
            inner: Rect::default(),
            slider_rect: Rect::default(),
            text_rect: Rect::default(),
            inner_text_rect: Rect::default(),
            combo_rect: Rect::default(),
            surface: Rect::default(),
            scroll_rect: Rect::default(),
        }
    }
}

pub(super) fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(800, 600), 1.0);
    let mut style = c.style().clone();
    style.motion.reduced_motion = true;
    c.set_style(style);
    c
}

fn options() -> Vec<ComboBoxOption<usize>> {
    (0..4)
        .map(|i| ComboBoxOption::new(i, i, format!("Option {i}")))
        .collect()
}

pub(super) fn draw(c: &mut Context, s: &mut Scene) {
    draw_at(c, s, Instant::now());
}

pub(super) fn draw_at(c: &mut Context, s: &mut Scene, now: Instant) {
    c.run_at(now, |c| {
        if c.input().keys_pressed.contains(&KeyCode::F3) {
            s.shortcut += 1;
        }
        Root::new().show(c, |ui| {
            if s.show_under {
                let under = ui.add(Button::new("Under").id_source("under"));
                s.under = under.rect;
                s.under_hovered = under.hovered;
                s.under_clicks += u32::from(under.clicked());
                let slider = ui.add(Slider::new(&mut s.slider, 0.0..=100.0).width(200.0));
                s.slider_rect = slider.rect;
                let text = ui.add(TextEdit::new(&mut s.under_text).id_source("under-text"));
                s.text_rect = text.rect;
                let combo = ui.add(
                    ComboBox::new(&mut s.under_combo, &options())
                        .id_source("under-combo")
                        .label("Below"),
                );
                s.under_combo_rect = combo.rect;
            }
            let area = ScrollArea::vertical()
                .id_source("under-scroll")
                .max_height(60.0)
                .show(ui, |ui| {
                    for i in 0..20 {
                        ui.label(format!("Row {i}"));
                    }
                });
            s.scroll_rect = area.viewport;
            s.scroll_offset = area.offset.y;
            let out = Modal::new("m")
                .dismiss_on_escape(s.escape)
                .dismiss_on_overlay(s.overlay)
                .show(ui, &mut s.open, |ui| {
                    let inner = ui.add(Button::new("Inner").id_source("inner"));
                    s.inner = inner.rect;
                    s.inner_clicks += u32::from(inner.clicked());
                    let text = ui.add(TextEdit::new(&mut s.inner_text).id_source("inner-text"));
                    s.inner_text_rect = text.rect;
                    let combo = ui.add(
                        ComboBox::new(&mut s.combo, &options())
                            .id_source("combo")
                            .label("Pick"),
                    );
                    s.combo_rect = combo.rect;
                    if s.long_body {
                        for i in 0..60 {
                            ui.label(format!("Line {i}"));
                        }
                    }
                    let nested = Modal::new("nested").show(ui, &mut s.nested, |ui| {
                        let b = ui.add(Button::new("Deep").id_source("deep"));
                        s.nested_clicks += u32::from(b.clicked());
                    });
                    if let Some(nested) = nested {
                        s.nested_closed.extend(nested.closed);
                    }
                });
            if let Some(out) = out {
                s.surface = out.rect;
                s.default_actions += u32::from(out.default_action);
                s.closed.extend(out.closed);
            }
        });
    });
}

pub(super) fn settle(c: &mut Context, s: &mut Scene) {
    for _ in 0..4 {
        draw(c, s);
    }
}

pub(super) fn open(c: &mut Context, s: &mut Scene) {
    s.open = true;
    settle(c, s);
}

pub(super) fn click(c: &mut Context, s: &mut Scene, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    draw(c, s);
    c.primary_button(ElementState::Released);
    draw(c, s);
}

pub(super) fn press(c: &mut Context, s: &mut Scene, code: KeyCode) -> bool {
    let consumed = c.on_key_event(code, ElementState::Pressed, false).consumed;
    draw(c, s);
    c.on_key_event(code, ElementState::Released, false);
    draw(c, s);
    consumed
}

pub(super) fn center(r: Rect) -> Vec2 {
    vec2((r.min.x + r.max.x) * 0.5, (r.min.y + r.max.y) * 0.5)
}

mod blocking;
mod closing;
mod dialogs;
mod focus;
mod layers;
mod underlay;
