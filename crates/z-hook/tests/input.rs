//! Who gets input: the overlay, the host, or both, in each mode.

use std::sync::{Arc, Mutex};
use z_hook::{testing::MockBackend, Delivery, Overlay, OverlayInput, OverlayOptions, ToggleKey};
use zaxis::{
    vec2,
    winit::{
        event::{ElementState, MouseButton},
        keyboard::{Key, KeyCode, ModifiersState, PhysicalKey},
    },
    Context, InputEvent, KeyInput, Rect, TextEdit, Window,
};

struct Rig {
    overlay: Overlay,
    backend: MockBackend,
    edit: Arc<Mutex<Option<Rect>>>,
    text: Arc<Mutex<String>>,
}

fn rig(mode: OverlayInput) -> Rig {
    let edit = Arc::new(Mutex::new(None));
    let text = Arc::new(Mutex::new(String::new()));
    let (e, t) = (Arc::clone(&edit), Arc::clone(&text));
    let options = OverlayOptions::default().input(mode).frame_budget(None);
    let overlay = Overlay::detached(options, move |ctx: &mut Context| {
        let mut buffer = t.lock().unwrap().clone();
        Window::new("Panel").show(ctx, |ui| {
            let response = ui.add(TextEdit::new(&mut buffer).id_source("field"));
            *e.lock().unwrap() = Some(response.rect);
        });
        *t.lock().unwrap() = buffer;
    });
    let mut backend = MockBackend::new([800, 600]);
    overlay.present(&mut backend);
    overlay.present(&mut backend);
    Rig {
        overlay,
        backend,
        edit,
        text,
    }
}

impl Rig {
    fn field(&self) -> zaxis::Vec2 {
        self.edit
            .lock()
            .unwrap()
            .expect("the field was built")
            .center()
    }

    fn send(&self, event: InputEvent) -> Delivery {
        self.overlay.input().send(event)
    }

    fn mouse(&self, at: zaxis::Vec2) -> Delivery {
        self.send(InputEvent::PointerMoved {
            x: f64::from(at.x),
            y: f64::from(at.y),
        })
    }

    fn frame(&mut self) {
        self.overlay.present(&mut self.backend);
    }
}

fn button(state: ElementState) -> InputEvent {
    InputEvent::Button {
        button: MouseButton::Left,
        state,
    }
}

fn key(code: KeyCode, state: ElementState, text: Option<&str>) -> InputEvent {
    InputEvent::Key(KeyInput {
        physical: PhysicalKey::Code(code),
        logical: Key::Character(text.unwrap_or("").into()),
        state,
        repeat: false,
        text: text.map(Into::into),
    })
}

const EMPTY: zaxis::Vec2 = vec2(790.0, 590.0);
const NEITHER: Delivery = Delivery {
    overlay: false,
    host: false,
};

#[test]
fn pass_through_gives_the_host_everything_and_the_overlay_a_copy() {
    let rig = rig(OverlayInput::PassThrough);
    assert_eq!(rig.mouse(rig.field()), Delivery::BOTH);
    assert_eq!(rig.send(button(ElementState::Pressed)), Delivery::BOTH);
    assert_eq!(rig.send(button(ElementState::Released)), Delivery::BOTH);
    assert_eq!(
        rig.send(key(KeyCode::KeyA, ElementState::Pressed, Some("a"))),
        Delivery::BOTH
    );
}

#[test]
fn capture_when_visible_takes_everything_but_old_releases() {
    let rig = rig(OverlayInput::CaptureWhenVisible);
    assert_eq!(rig.mouse(EMPTY), Delivery::OVERLAY);
    assert_eq!(rig.send(button(ElementState::Pressed)), Delivery::OVERLAY);
    assert_eq!(rig.send(button(ElementState::Released)), Delivery::OVERLAY);
    // A release whose press the host saw (before the overlay showed) must reach the host.
    let old = key(KeyCode::KeyW, ElementState::Released, None);
    assert!(
        rig.send(old).host,
        "never taken by the overlay, so the host needs it"
    );
}

#[test]
fn capture_when_focused_takes_what_a_control_uses() {
    let mut rig = rig(OverlayInput::CaptureWhenFocused);
    assert_eq!(
        rig.mouse(EMPTY),
        Delivery::BOTH,
        "empty space belongs to the host too"
    );
    assert!(rig.mouse(rig.field()).overlay && !rig.mouse(rig.field()).host);
    let press = rig.send(button(ElementState::Pressed));
    assert_eq!((press.overlay, press.host), (true, false));
    let release = rig.send(button(ElementState::Released));
    assert_eq!(
        (release.overlay, release.host),
        (true, false),
        "the release of a taken press"
    );
    rig.frame();
    rig.frame();
    let typed = rig.send(key(KeyCode::KeyA, ElementState::Pressed, Some("a")));
    assert_eq!(
        (typed.overlay, typed.host),
        (true, false),
        "a focused field takes keys"
    );
    assert!(
        !rig.send(key(KeyCode::KeyA, ElementState::Released, None))
            .host
    );
    rig.frame();
    assert_eq!(*rig.text.lock().unwrap(), "a");
}

#[test]
fn keys_go_to_the_host_while_no_control_has_focus() {
    let rig = rig(OverlayInput::CaptureWhenFocused);
    let delivery = rig.send(key(KeyCode::KeyW, ElementState::Pressed, Some("w")));
    assert!(delivery.host, "no focus: movement keys stay with the game");
    assert!(
        rig.send(key(KeyCode::KeyW, ElementState::Released, None))
            .host
    );
}

#[test]
fn a_hidden_overlay_gives_the_host_everything() {
    let rig = rig(OverlayInput::CaptureWhenVisible);
    rig.overlay.set_visible(false);
    assert_eq!(rig.mouse(rig.field()), Delivery::HOST);
    assert_eq!(rig.send(button(ElementState::Pressed)), Delivery::HOST);
}

#[test]
fn the_toggle_key_shows_and_hides_and_is_not_forwarded() {
    let rig = rig(OverlayInput::CaptureWhenFocused);
    let press = key(KeyCode::F10, ElementState::Pressed, None);
    assert_eq!(rig.send(press.clone()), NEITHER);
    assert!(!rig.overlay.is_visible());
    assert!(
        !rig.send(key(KeyCode::F10, ElementState::Released, None))
            .host,
        "its release too"
    );
    assert_eq!(rig.send(press.clone()), NEITHER);
    assert!(rig.overlay.is_visible());
}

#[test]
fn a_toggle_with_modifiers_needs_them() {
    let options = OverlayOptions::default()
        .toggle(Some(
            ToggleKey::new(KeyCode::KeyO).with_modifiers(ModifiersState::CONTROL),
        ))
        .frame_budget(None);
    let overlay = Overlay::detached(options, |_: &mut Context| {});
    let sink = overlay.input();
    sink.send(key(KeyCode::KeyO, ElementState::Pressed, None));
    assert!(overlay.is_visible(), "without Ctrl it is an ordinary key");
    sink.send(InputEvent::Modifiers(ModifiersState::CONTROL));
    sink.send(key(KeyCode::KeyO, ElementState::Pressed, None));
    assert!(!overlay.is_visible());
}

#[test]
fn pass_through_forwards_the_toggle_to_the_host() {
    let options = OverlayOptions::default().input(OverlayInput::PassThrough);
    let overlay = Overlay::detached(options, |_: &mut Context| {});
    let delivery = overlay
        .input()
        .send(key(KeyCode::F10, ElementState::Pressed, None));
    assert!(delivery.host && !overlay.is_visible());
}

#[test]
fn input_from_another_thread_waits_for_the_next_frame() {
    let mut rig = rig(OverlayInput::CaptureWhenFocused);
    let sink = rig.overlay.input();
    let at = rig.field();
    std::thread::spawn(move || {
        sink.send(InputEvent::PointerMoved {
            x: f64::from(at.x),
            y: f64::from(at.y),
        });
    })
    .join()
    .unwrap();
    rig.frame();
    // The frame applied the queued move: the policy now knows the pointer is over the field.
    let press = rig.send(button(ElementState::Pressed));
    assert!(!press.host);
}
