//! Nested popups: a popup opened from inside another keeps its parent.
use crate::prelude::*;
use winit::{dpi::PhysicalSize, event::ElementState};
use zaxis::{vec2, Button, ComboBox, ComboBoxOption, Modal, Popup, Rect, Root, TextEdit};

mod hierarchy;
mod input;
mod lifecycle;
mod placement;

pub(crate) fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::from_min_size(vec2(x, y), vec2(w, h))
}

#[derive(Default)]
pub(crate) struct Scene {
    pub a: bool,
    pub b: bool,
    pub child: bool,
    pub child2: bool,
    pub grand: bool,
    pub modal: bool,
    /// Build the parent's popup at all (a removed control builds nothing).
    pub build_a: bool,
    /// Build the child's popup (false: the control that owns it is gone).
    pub build_child: bool,
    pub combo_in_a: bool,
    pub popup_in_modal: bool,
    pub under_clicks: u32,
    pub plain_clicks: u32,
    pub grand_clicks: u32,
    pub combo: Option<usize>,
    pub text: String,
    pub a_btn: Rect,
    pub b_btn: Rect,
    pub under: Rect,
    pub child_btn: Rect,
    pub child2_btn: Rect,
    pub plain: Rect,
    pub grand_btn: Rect,
    pub grand_inner: Rect,
    pub combo_rect: Rect,
    pub text_rect: Rect,
    pub a_rect: Option<Rect>,
    pub child_rect: Option<Rect>,
    pub grand_rect: Option<Rect>,
    pub modal_btn: Rect,
    pub a_pos: Vec2,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            build_a: true,
            build_child: true,
            a_pos: vec2(20.0, 20.0),
            ..Default::default()
        }
    }
}

pub(crate) fn setup() -> Context {
    let mut c = Context::new();
    c.set_viewport(PhysicalSize::new(700, 500), 1.0);
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

fn toggle(flag: &mut bool, response: &zaxis::Response) {
    if response.clicked() {
        *flag = !*flag;
    }
}

fn at_button(ui: &mut zaxis::Ui<'_>, name: &str, at: Rect) -> zaxis::Response {
    ui.at(name, at, |ui| ui.add(Button::new(name).id_source(name)))
}

fn grand_popup(ui: &mut zaxis::Ui<'_>, s: &mut Scene) {
    let grand_btn = ui.add(Button::new("grand"));
    s.grand_btn = grand_btn.rect;
    toggle(&mut s.grand, &grand_btn);
    let anchor = grand_btn.rect;
    let mut open = s.grand;
    let shown = Popup::new("grand", anchor)
        .size(vec2(120.0, 60.0))
        .show(ui, &mut open, |ui| {
            let inner = ui.add(Button::new("inner").id_source("inner"));
            s.grand_inner = inner.rect;
            s.grand_clicks += u32::from(inner.clicked());
        });
    s.grand = open;
    s.grand_rect = shown.map(|o| o.rect);
}

fn child_popup(ui: &mut zaxis::Ui<'_>, s: &mut Scene) {
    let (btn, btn2) = ui.horizontal(|ui| {
        let btn = ui.add(Button::new("child"));
        let btn2 = ui.add(Button::new("child2"));
        let plain = ui.add(Button::new("plain"));
        s.plain = plain.rect;
        s.plain_clicks += u32::from(plain.clicked());
        (btn, btn2)
    });
    s.child_btn = btn.rect;
    toggle(&mut s.child, &btn);
    s.child2_btn = btn2.rect;
    toggle(&mut s.child2, &btn2);
    if s.combo_in_a {
        let combo = ui.add(ComboBox::new(&mut s.combo, &options()).id_source("combo"));
        s.combo_rect = combo.rect;
        let text = ui.add(TextEdit::new(&mut s.text).id_source("text"));
        s.text_rect = text.rect;
    }
    if !s.build_child {
        return;
    }
    let mut open = s.child;
    let shown = Popup::new("child", btn.rect)
        .size(vec2(200.0, 120.0))
        .show(ui, &mut open, |ui| grand_popup(ui, s));
    s.child = open;
    s.child_rect = shown.map(|o| o.rect);
    let mut open = s.child2;
    Popup::new("child2", btn2.rect)
        .size(vec2(200.0, 80.0))
        .show(ui, &mut open, |ui| {
            ui.label("second");
        });
    s.child2 = open;
}

/// One pass. Popup A and B hang off two root buttons; "Under" sits clear of both.
pub(crate) fn pass(c: &mut Context, s: &mut Scene) {
    c.run(|c| {
        Root::new().show(c, |ui| {
            let a = at_button(ui, "a", Rect::from_min_size(s.a_pos, vec2(80.0, 28.0)));
            s.a_btn = a.rect;
            toggle(&mut s.a, &a);
            let b = at_button(ui, "b", rect(140.0, 20.0, 80.0, 28.0));
            s.b_btn = b.rect;
            toggle(&mut s.b, &b);
            let under = at_button(ui, "under", rect(400.0, 300.0, 80.0, 28.0));
            s.under = under.rect;
            s.under_clicks += u32::from(under.clicked());
            if s.build_a {
                let mut open = s.a;
                let shown =
                    Popup::new("a", a.rect)
                        .size(vec2(240.0, 200.0))
                        .show(ui, &mut open, |ui| child_popup(ui, s));
                s.a = open;
                s.a_rect = shown.map(|o| o.rect);
            }
            let mut open = s.b;
            Popup::new("b", b.rect)
                .size(vec2(160.0, 60.0))
                .show(ui, &mut open, |ui| {
                    ui.label("b");
                });
            s.b = open;
            if s.modal {
                let mut open = s.modal;
                Modal::new("m").show(ui, &mut open, |ui| {
                    let btn = ui.add(Button::new("open").id_source("modal-open"));
                    s.modal_btn = btn.rect;
                    toggle(&mut s.popup_in_modal, &btn);
                    let mut inner = s.popup_in_modal;
                    Popup::new("in-modal", btn.rect)
                        .size(vec2(160.0, 90.0))
                        .show(ui, &mut inner, |ui| grand_popup(ui, s));
                    s.popup_in_modal = inner;
                });
                s.modal = open;
            }
        });
    });
}

/// Run until the published hit regions follow the model.
pub(crate) fn settle(c: &mut Context, s: &mut Scene) {
    for _ in 0..3 {
        pass(c, s);
    }
}

pub(crate) fn click(c: &mut Context, s: &mut Scene, p: Vec2) {
    c.move_pointer(p);
    c.primary_button(ElementState::Pressed);
    c.primary_button(ElementState::Released);
    pass(c, s);
}

pub(crate) fn tap(c: &mut Context, s: &mut Scene, at: impl Fn(&Scene) -> Rect) {
    let p = at(s).center();
    click(c, s, p);
}

pub(crate) fn key(c: &mut Context, code: KeyCode) -> bool {
    let consumed = c.on_key_event(code, ElementState::Pressed, false).consumed;
    c.on_key_event(code, ElementState::Released, false);
    consumed
}

pub(crate) fn ids(c: &Context) -> Vec<Id> {
    c.probe().popups.iter().map(|p| p.id).collect()
}

pub(crate) fn center(rect: Rect) -> Vec2 {
    rect.center()
}

/// Opens A, then child, then grand, one click each.
pub(crate) fn focus_rect(c: &mut Context, rect: Rect) {
    let id = c
        .probe()
        .previous_hits
        .iter()
        .find(|h| h.rect == rect)
        .map(|h| h.id);
    c.request_focus(id.expect("a control at that rect"));
}

/// Like [`open_chain`], with keyboard focus on each trigger before it is clicked.
pub(crate) fn open_chain_focused(c: &mut Context, s: &mut Scene, depth: usize) {
    settle(c, s);
    focus_rect(c, s.a_btn);
    tap(c, s, |s| s.a_btn);
    pass(c, s);
    if depth > 1 {
        focus_rect(c, s.child_btn);
        tap(c, s, |s| s.child_btn);
        pass(c, s);
    }
    if depth > 2 {
        focus_rect(c, s.grand_btn);
        tap(c, s, |s| s.grand_btn);
        pass(c, s);
    }
}

pub(crate) fn open_chain(c: &mut Context, s: &mut Scene, depth: usize) {
    settle(c, s);
    click(c, s, center(s.a_btn));
    pass(c, s);
    if depth > 1 {
        click(c, s, center(s.child_btn));
        pass(c, s);
    }
    if depth > 2 {
        click(c, s, center(s.grand_btn));
        pass(c, s);
    }
}
