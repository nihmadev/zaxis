//! Action registry and keymap on the real input path: `count` actions (500 is the reference
//! size), each with its own shortcut, behind a menu bar and a row of buttons. Idle frames,
//! a key that matches nothing, a key that runs an action, a two-stroke chord, and rebinding
//! an action. `count` is the number of actions. Keys go through `Context::on_input`; state
//! and counters are asserted outside the timed section.
use zaxis::winit::{
    event::ElementState,
    keyboard::{Key, KeyCode, ModifiersState, PhysicalKey},
};
use zaxis::Instant;
use zaxis::{
    Action, Actions, Button, Context, InputEvent, KeyInput, MenuBar, MenuItem, Mods, Platform,
    Root, Stroke,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionsCase {
    /// Nothing changed: no repaint, no new geometry, the registry costs nothing.
    Idle,
    /// A key no action is bound to: the lookup finds nothing and the key stays unused.
    Miss,
    /// A key bound to an action, rotating through all of them: one event per press.
    Hit,
    /// The two strokes of a chord: the first waits, the second runs the action.
    Chord,
    /// One action rebound to another chord and back: the keymap is rebuilt.
    Rebind,
}

impl ActionsCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Idle => "actions_idle",
            Self::Miss => "actions_key_miss",
            Self::Hit => "actions_key_hit",
            Self::Chord => "actions_chord",
            Self::Rebind => "actions_rebind",
        }
    }
    pub fn interactive(self) -> bool {
        self != Self::Idle
    }
}

const KEYS: [KeyCode; 47] = [
    KeyCode::KeyA,
    KeyCode::KeyB,
    KeyCode::KeyC,
    KeyCode::KeyD,
    KeyCode::KeyE,
    KeyCode::KeyF,
    KeyCode::KeyG,
    KeyCode::KeyH,
    KeyCode::KeyI,
    KeyCode::KeyJ,
    KeyCode::KeyK,
    KeyCode::KeyL,
    KeyCode::KeyM,
    KeyCode::KeyN,
    KeyCode::KeyO,
    KeyCode::KeyP,
    KeyCode::KeyQ,
    KeyCode::KeyR,
    KeyCode::KeyS,
    KeyCode::KeyT,
    KeyCode::KeyU,
    KeyCode::KeyV,
    KeyCode::KeyW,
    KeyCode::KeyX,
    KeyCode::KeyY,
    KeyCode::KeyZ,
    KeyCode::Digit0,
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
    KeyCode::Digit7,
    KeyCode::Digit8,
    KeyCode::Digit9,
    KeyCode::F1,
    KeyCode::F2,
    KeyCode::F3,
    KeyCode::F4,
    KeyCode::F5,
    KeyCode::F6,
    KeyCode::F7,
    KeyCode::F8,
    KeyCode::F9,
    KeyCode::F10,
    KeyCode::F11,
];
/// 14 modifier sets times 47 keys: that many distinct shortcuts.
const MAX_ACTIONS: usize = 14 * 47;
/// Every `CHORD_EVERY`th action is a two-stroke chord.
const CHORD_EVERY: usize = 50;
const SECOND: KeyCode = KeyCode::F12;

/// The shortcut of action `index`; no two are alike.
fn stroke(index: usize) -> (ModifiersState, KeyCode) {
    let mut state = ModifiersState::empty();
    let set = index / KEYS.len() % 14;
    for (bit, flag) in [
        ModifiersState::CONTROL,
        ModifiersState::ALT,
        ModifiersState::SHIFT,
    ]
    .into_iter()
    .enumerate()
    {
        if (set % 7 + 1) >> bit & 1 == 1 {
            state |= flag;
        }
    }
    if set >= 7 {
        state |= ModifiersState::SUPER;
    }
    (state, KEYS[index % KEYS.len()])
}

fn mods(state: ModifiersState) -> Mods {
    Mods::from_state(state)
}

fn chord_of(index: usize) -> zaxis::Chord {
    let (state, key) = stroke(index);
    let first = Stroke::new(mods(state), key);
    if index % CHORD_EVERY == CHORD_EVERY - 1 {
        first.then(Stroke::new(Mods::NONE, SECOND))
    } else {
        first.into()
    }
}

fn registry(count: usize) -> Actions {
    let mut actions = Actions::new();
    for index in 0..count {
        let action = Action::new(("bench", index), format!("Action {index}"))
            .group(format!("Group {}", index / 20))
            .shortcut(chord_of(index));
        actions.insert(action);
    }
    actions.keymap_mut().set_platform(Platform::Linux);
    actions
}

pub struct Probe {
    kind: ActionsCase,
    count: usize,
    now: Instant,
    step: usize,
    /// The action the last input stands for, and whether its event was taken.
    expected: Option<usize>,
    taken: Vec<usize>,
    consumed: Option<bool>,
    rebound: usize,
    revision: u64,
    tessellations: u64,
}

impl Probe {
    pub fn new(context: &mut Context, kind: ActionsCase, count: usize) -> Self {
        let count = count.clamp(CHORD_EVERY, MAX_ACTIONS);
        let actions = registry(count);
        assert!(
            actions.keymap().conflicts().is_empty(),
            "bench shortcuts are unique"
        );
        assert!(
            !actions.keymap().bindings().any(|b| b.chord == miss_chord()),
            "the missing key is unbound"
        );
        context.set_actions(actions);
        let mut probe = Self {
            kind,
            count,
            now: Instant::now(),
            step: 0,
            expected: None,
            taken: Vec::new(),
            consumed: None,
            rebound: 0,
            revision: 0,
            tessellations: 0,
        };
        for _ in 0..3 {
            probe.build(context);
        }
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.step = step;
        self.now += std::time::Duration::from_millis(20);
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        self.expected = None;
        self.consumed = None;
        match self.kind {
            ActionsCase::Idle => {}
            ActionsCase::Miss => {
                self.consumed = Some(send(
                    context,
                    ModifiersState::CONTROL | ModifiersState::SUPER,
                    KeyCode::F24,
                ));
            }
            ActionsCase::Hit => {
                let index = hit_index(step, self.count);
                let (state, key) = stroke(index);
                self.expected = Some(index);
                self.consumed = Some(send(context, state, key));
            }
            ActionsCase::Chord => {
                let index = chord_index(step, self.count);
                let (state, key) = stroke(index);
                self.expected = Some(index);
                let first = send(context, state, key);
                let second = send(context, ModifiersState::empty(), SECOND);
                self.consumed = Some(first && second);
            }
            ActionsCase::Rebind => {
                let index = step % self.count;
                let moved = step.is_multiple_of(2);
                let chord = if moved {
                    rebound_chord(index)
                } else {
                    chord_of(index)
                };
                let clashes = context
                    .actions()
                    .keymap_mut()
                    .rebind(("bench", index), Some(chord));
                self.rebound = index;
                self.consumed = Some(clashes.is_empty());
            }
        }
    }

    pub fn build(&mut self, context: &mut Context) {
        self.now += std::time::Duration::from_millis(1);
        self.taken.clear();
        let (count, expected) = (self.count, self.expected);
        let mut taken = Vec::new();
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                // The application tells which actions are available now.
                for index in (0..count).step_by(5) {
                    ui.actions().set_enabled(("bench", index), index % 10 != 0);
                }
                let items = [
                    MenuItem::submenu(
                        "File",
                        (0..8)
                            .map(|i| MenuItem::action(("bench", i)))
                            .collect::<Vec<_>>(),
                    ),
                    MenuItem::submenu(
                        "Edit",
                        (8..16)
                            .map(|i| MenuItem::action(("bench", i)))
                            .collect::<Vec<_>>(),
                    ),
                ];
                MenuBar::new("bar", &items).show(ui);
                ui.horizontal(|ui| {
                    for index in 16..20 {
                        ui.add(Button::action(("bench", index)));
                    }
                });
                if let Some(index) = expected {
                    if ui.actions().triggered(("bench", index)) {
                        taken.push(index);
                    }
                }
            });
        });
        self.taken = taken;
    }

    pub fn verify(&self, context: &Context) {
        match self.kind {
            ActionsCase::Idle => {
                assert!(!context.needs_repaint(), "an unchanged form needs no frame");
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
            ActionsCase::Miss => {
                assert_eq!(self.consumed, Some(false), "nothing used the key");
                assert!(self.taken.is_empty());
            }
            ActionsCase::Hit => {
                let index = self.expected.unwrap();
                let enabled = !index.is_multiple_of(10);
                assert_eq!(
                    self.consumed,
                    Some(enabled),
                    "an enabled action uses its key, a disabled one leaves it"
                );
                assert_eq!(self.taken, if enabled { vec![index] } else { vec![] });
            }
            ActionsCase::Chord => {
                let index = self.expected.unwrap();
                assert_eq!(self.consumed, Some(true), "both strokes were used");
                assert_eq!(self.taken, vec![index]);
                assert!(context.pending_chord().is_none(), "the chord finished");
            }
            ActionsCase::Rebind => {
                assert_eq!(self.consumed, Some(true), "no clash");
                let wanted = if self.step.is_multiple_of(2) {
                    rebound_chord(self.rebound)
                } else {
                    chord_of(self.rebound)
                };
                let bindings: Vec<_> = context
                    .action_registry()
                    .keymap()
                    .bindings_of(("bench", self.rebound))
                    .into_iter()
                    .map(|b| b.chord.clone())
                    .collect();
                assert_eq!(bindings, vec![wanted]);
            }
        }
    }
}

fn miss_chord() -> zaxis::Chord {
    Stroke::new(Mods::CTRL | Mods::META, KeyCode::F24).into()
}

/// A chord no other action has, to move an action to.
fn rebound_chord(index: usize) -> zaxis::Chord {
    let (state, key) = stroke(index);
    Stroke::new(Mods::NONE, KeyCode::F23).then(Stroke::new(mods(state), key))
}

/// Single-stroke actions only, rotating through all of them.
fn hit_index(step: usize, count: usize) -> usize {
    let index = step.wrapping_mul(7) % count;
    if index % CHORD_EVERY == CHORD_EVERY - 1 {
        index - 1
    } else {
        index
    }
}

fn chord_index(step: usize, count: usize) -> usize {
    let chords = count / CHORD_EVERY;
    (step % chords.max(1) + 1) * CHORD_EVERY - 1
}

/// Press and release one key with the modifiers held, as a window sends them. Returns
/// whether the press was used.
fn send(context: &mut Context, modifiers: ModifiersState, code: KeyCode) -> bool {
    context.on_input(InputEvent::Modifiers(modifiers));
    let letter = format!("{code:?}")
        .strip_prefix("Key")
        .filter(|rest| rest.len() == 1)
        .map(|rest| Key::Character(rest.to_lowercase().into()));
    let event = |state| {
        InputEvent::Key(KeyInput {
            physical: PhysicalKey::Code(code),
            logical: letter.clone().unwrap_or(Key::Unidentified(
                zaxis::winit::keyboard::NativeKey::Unidentified,
            )),
            state,
            repeat: false,
            text: None,
        })
    };
    let used = context.on_input(event(ElementState::Pressed)).consumed;
    context.on_input(event(ElementState::Released));
    context.on_input(InputEvent::Modifiers(ModifiersState::empty()));
    used
}
