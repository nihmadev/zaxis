//! Keys addressed to external controls and focus groups on the real input path:
//! `Context::on_input`, then `Context::run`. `count` is the number of controls; each is a
//! region written against the public API. The idle pair is the same scene without and with
//! key claims and one focus group. Keys are synthetic: the figure is dispatcher to model and
//! frame, not an operating system to display. Values, order, focus, bounds and the bounded
//! counters of `Context::input_stats` are asserted outside the timed section.
use zaxis::winit::{
    event::ElementState,
    keyboard::{Key, KeyCode, NativeKey, PhysicalKey},
};
use zaxis::{
    vec2, Color, Context, FocusGroup, Id, InputEvent, Instant, KeyInput, KeyInterest, Rect,
    Response, Root, Sense, Shape, Ui,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeysCase {
    /// Controls without claims or a group.
    IdleOff,
    /// The same controls with claims in one group: nothing changes, nothing is redrawn.
    IdleOn,
    /// A claimed key reaches the focused control and changes its value.
    Claimed,
    /// A key nobody claimed stays unused.
    Unclaimed,
    /// An arrow moves focus within one group of every control.
    GroupStep,
    /// Eight claimed keys arrive between two passes and are applied in order.
    Burst,
    /// An arrow within one of many small groups.
    ManyGroups,
    /// Members are disabled and reordered between steps, then an arrow moves focus.
    Reorder,
    /// Controls are mounted and unmounted; every table stays bounded.
    Lifecycle,
}

impl KeysCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::IdleOff => "keys_idle_off",
            Self::IdleOn => "keys_idle_on",
            Self::Claimed => "keys_claimed",
            Self::Unclaimed => "keys_unclaimed",
            Self::GroupStep => "keys_group_step",
            Self::Burst => "keys_burst",
            Self::ManyGroups => "keys_many_groups",
            Self::Reorder => "keys_reorder",
            Self::Lifecycle => "keys_lifecycle",
        }
    }
    pub fn interactive(self) -> bool {
        !matches!(self, Self::IdleOff | Self::IdleOn | Self::Lifecycle)
    }
    fn grouped(self) -> bool {
        self != Self::IdleOff
    }
}

/// Controls in one row of the scene.
const ROW: usize = 24;
/// Controls in one group of the many-groups case.
const SMALL: usize = 10;
const BURST: [KeyCode; 8] = [
    KeyCode::KeyQ,
    KeyCode::KeyE,
    KeyCode::KeyQ,
    KeyCode::KeyE,
    KeyCode::KeyQ,
    KeyCode::KeyE,
    KeyCode::KeyQ,
    KeyCode::KeyE,
];

/// The application's model: one number per control, whether it is enabled, and the order
/// the controls are shown in.
struct Model {
    values: Vec<u64>,
    enabled: Vec<bool>,
    order: Vec<usize>,
}

pub struct Probe {
    kind: KeysCase,
    count: usize,
    now: Instant,
    model: Model,
    /// How many controls are built (the lifecycle case varies it).
    mounted: usize,
    /// What the last build saw: rectangles by control, the control with focus, the first id.
    rects: Vec<Rect>,
    /// Where the controls are visible: the clip of the root.
    clip: Rect,
    focus: Option<usize>,
    first: Option<Id>,
    /// Controls that are not clipped away, in display order, as the first build found them.
    visible: usize,
    /// The control the model disables for one step (the reorder case).
    disabled: Option<usize>,
    /// What the last input should have produced.
    expect_value: Option<(usize, u64)>,
    expect_focus: Option<usize>,
    consumed: Option<bool>,
    revision: u64,
    tessellations: u64,
}

impl Probe {
    pub fn new(context: &mut Context, kind: KeysCase, count: usize) -> Self {
        let count = count.max(SMALL * 2);
        let mut probe = Self {
            kind,
            count,
            now: Instant::now(),
            model: Model {
                values: vec![1; count],
                enabled: vec![true; count],
                order: (0..count).collect(),
            },
            mounted: count,
            rects: vec![Rect::default(); count],
            clip: Rect::default(),
            focus: None,
            first: None,
            visible: 0,
            disabled: None,
            expect_value: None,
            expect_focus: None,
            consumed: None,
            revision: 0,
            tessellations: 0,
        };
        for _ in 0..3 {
            probe.build(context);
        }
        probe.visible = probe
            .model
            .order
            .iter()
            .take_while(|&&i| !probe.rects[i].intersect(probe.clip).is_empty())
            .count();
        assert!(
            probe.visible >= SMALL,
            "the scene shows at least one small group"
        );
        if kind.grouped() {
            context.request_focus(probe.first.expect("the first control was built"));
            probe.build(context);
            probe.build(context);
            assert_eq!(probe.focus, Some(0), "focus starts on the first control");
        }
        probe
    }

    pub fn input(&mut self, context: &mut Context, step: usize) {
        self.revision = context.draw_data().revision;
        self.tessellations = context.cache_stats().tessellated_elements;
        self.expect_value = None;
        self.expect_focus = None;
        self.consumed = None;
        let current = self.focus.unwrap_or(0);
        match self.kind {
            KeysCase::IdleOff | KeysCase::IdleOn => {}
            KeysCase::Claimed => {
                self.consumed = Some(send(context, KeyCode::KeyQ));
                self.expect_value = Some((current, self.model.values[current].wrapping_add(1)));
            }
            KeysCase::Unclaimed => self.consumed = Some(send(context, KeyCode::KeyW)),
            KeysCase::GroupStep => {
                self.consumed = Some(send(context, KeyCode::ArrowRight));
                self.expect_focus = Some((position(&self.model, current) + 1) % self.visible);
            }
            KeysCase::ManyGroups => {
                self.consumed = Some(send(context, KeyCode::ArrowRight));
                let group = current / SMALL * SMALL;
                self.expect_focus = Some(group + (current + 1 - group) % SMALL);
            }
            KeysCase::Burst => {
                let mut value = self.model.values[current];
                for key in BURST {
                    self.consumed = Some(send(context, key));
                    value = apply(value, key);
                }
                self.expect_value = Some((current, value));
            }
            KeysCase::Reorder => {
                let at = position(&self.model, current);
                let next = (1..=self.visible)
                    .map(|n| (at + n) % self.visible)
                    .find(|&p| self.model.enabled[self.model.order[p]])
                    .expect("an enabled control exists");
                self.consumed = Some(send(context, KeyCode::ArrowRight));
                self.expect_focus = Some(self.model.order[next]);
                // The application changes its model for the next pass: the control disabled
                // last time is enabled again, or another one is disabled, and two others
                // swap places.
                if let Some(control) = self.disabled.take() {
                    self.model.enabled[control] = true;
                } else {
                    let control = self.model.order[(next + 3 + step % 5) % self.visible];
                    if control != self.model.order[next] {
                        self.model.enabled[control] = false;
                        self.disabled = Some(control);
                    }
                }
                let (a, b) = ((next + 1) % self.visible, (next + 2) % self.visible);
                if a != next && b != next {
                    self.model.order.swap(a, b);
                }
            }
            KeysCase::Lifecycle => {
                self.mounted = if step % 2 == 0 {
                    self.count
                } else {
                    self.count / 2
                };
            }
        }
    }

    pub fn build(&mut self, context: &mut Context) {
        self.now += std::time::Duration::from_millis(1);
        let (kind, mounted) = (self.kind, self.mounted);
        let Self {
            model,
            rects,
            first,
            clip,
            ..
        } = self;
        let mut focus = None;
        context.run_at(self.now, |context| {
            Root::new().show(context, |ui| {
                *clip = ui.clip_rect();
                let shown: Vec<usize> = model
                    .order
                    .iter()
                    .copied()
                    .filter(|&i| i < mounted)
                    .collect();
                let mut at = 0;
                let mut note = |index: usize, response: &Response, first: &mut Option<Id>| {
                    rects[index] = response.rect;
                    if response.has_focus {
                        focus = Some(index);
                    }
                    if index == 0 {
                        *first = Some(response.id);
                    }
                };
                match kind {
                    KeysCase::IdleOff => {
                        for row in shown.chunks(ROW) {
                            ui.horizontal(|ui| {
                                for &i in row {
                                    let r = control(ui, i, model, false);
                                    note(i, &r, first);
                                }
                            });
                        }
                    }
                    KeysCase::ManyGroups => {
                        for row in shown.chunks(ROW) {
                            ui.horizontal(|ui| {
                                for small in row.chunks(SMALL) {
                                    FocusGroup::new(("small", at)).wrap(true).show(ui, |ui| {
                                        for &i in small {
                                            let r = control(ui, i, model, true);
                                            note(i, &r, first);
                                        }
                                    });
                                    at += 1;
                                }
                            });
                        }
                    }
                    _ => {
                        FocusGroup::new("all").wrap(true).show(ui, |ui| {
                            for row in shown.chunks(ROW) {
                                ui.horizontal(|ui| {
                                    for &i in row {
                                        let r = control(ui, i, model, true);
                                        note(i, &r, first);
                                    }
                                });
                            }
                        });
                    }
                }
            });
        });
        self.focus = focus.or(self.focus.filter(|_| self.kind == KeysCase::IdleOff));
    }

    pub fn verify(&self, context: &Context) {
        let stats = context.input_stats();
        assert_eq!(
            stats.key_events_pending, 0,
            "every event was taken or dropped"
        );
        assert_eq!(stats.keys_owned, 0, "no key is left held");
        assert_eq!(stats.key_events_dropped, 0, "the queue never overflowed");
        let disabled = self.model.enabled[..self.mounted]
            .iter()
            .filter(|enabled| !**enabled)
            .count();
        let claims = if self.kind.grouped() {
            self.mounted - disabled
        } else {
            0
        };
        assert_eq!(stats.key_claims, claims, "one claim per enabled control");
        let groups = match self.kind {
            KeysCase::IdleOff => 0,
            KeysCase::ManyGroups => self.small_groups_in_view(),
            _ => 1,
        };
        assert_eq!(stats.focus_groups, groups, "groups of the last pass only");
        if let Some(value) = self.expect_value {
            assert_eq!(
                self.model.values[value.0], value.1,
                "the value follows the keys"
            );
        }
        if let Some(index) = self.expect_focus {
            assert_eq!(self.focus, Some(index), "focus moved to the neighbour");
            assert!(
                !self.rects[index].intersect(self.clip).is_empty(),
                "and its bounds are on screen"
            );
        }
        match self.kind {
            KeysCase::IdleOff | KeysCase::IdleOn => {
                assert!(
                    !context.needs_repaint(),
                    "an unchanged scene needs no frame"
                );
                assert_eq!(self.revision, context.draw_data().revision);
                assert_eq!(
                    self.tessellations,
                    context.cache_stats().tessellated_elements
                );
            }
            KeysCase::Unclaimed => assert_eq!(self.consumed, Some(false), "nobody used W"),
            KeysCase::Claimed | KeysCase::Burst | KeysCase::GroupStep | KeysCase::ManyGroups => {
                assert_eq!(self.consumed, Some(true), "the key was used on arrival")
            }
            KeysCase::Reorder => {
                assert_eq!(self.consumed, Some(true));
                let focused = self.focus.expect("focus stays on an enabled control");
                assert!(self.model.enabled[focused]);
            }
            KeysCase::Lifecycle => {
                assert!(stats.focus_group_members <= self.mounted + 1);
                assert_eq!(self.focus, Some(0), "focus stays on the first control");
            }
        }
    }
}

impl Probe {
    /// Small groups with a member that is not clipped away: only those are published.
    fn small_groups_in_view(&self) -> usize {
        let (mut groups, mut shown) = (0, 0);
        for row in (0..self.mounted).step_by(ROW) {
            let len = ROW.min(self.mounted - row);
            for start in (0..len).step_by(SMALL) {
                if shown < self.visible {
                    groups += 1;
                }
                shown += SMALL.min(len - start);
            }
        }
        groups
    }
}

fn position(model: &Model, control: usize) -> usize {
    model
        .order
        .iter()
        .position(|&i| i == control)
        .expect("the control is shown")
}

/// Q adds one and E doubles, so the order of the keys decides the result.
fn apply(value: u64, key: KeyCode) -> u64 {
    if key == KeyCode::KeyQ {
        value.wrapping_add(1)
    } else {
        value.wrapping_mul(2)
    }
}

fn control(ui: &mut Ui<'_>, index: usize, model: &mut Model, claims: bool) -> Response {
    let rect = ui.allocate_space(vec2(28.0, 18.0));
    let enabled = model.enabled[index];
    let response = ui.add_enabled_ui(enabled, |ui| {
        ui.interact(rect, ("k", index), Sense::CLICK | Sense::FOCUS)
    });
    if claims {
        ui.claim_keys(
            &response,
            KeyInterest::keys(&[KeyCode::KeyQ, KeyCode::KeyE]),
        );
        for key in ui.take_keys(&response) {
            model.values[index] = apply(model.values[index], key.code);
        }
    }
    let shade = 40 + (model.values[index] % 8) as u8 * 12;
    ui.paint(Shape::rect(rect, Color::gray(shade)));
    response
}

/// Press and release one key as a window sends it; returns whether the press was used.
fn send(context: &mut Context, code: KeyCode) -> bool {
    let logical = match code {
        KeyCode::ArrowRight => Key::Named(zaxis::winit::keyboard::NamedKey::ArrowRight),
        _ => format!("{code:?}")
            .strip_prefix("Key")
            .map(|letter| Key::Character(letter.to_lowercase().into()))
            .unwrap_or(Key::Unidentified(NativeKey::Unidentified)),
    };
    let event = |state| {
        InputEvent::Key(KeyInput {
            physical: PhysicalKey::Code(code),
            logical: logical.clone(),
            state,
            repeat: false,
            text: None,
        })
    };
    let used = context.on_input(event(ElementState::Pressed)).consumed;
    context.on_input(event(ElementState::Released));
    used
}
