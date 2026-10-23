//! Public `Wrap` paths: idle reuse, narrow and wide areas, resize, one changed item, mixed
//! heights, compound controls, input on a wrapped item, reorder and lifecycle, next to a plain
//! row. Every item is built each pass (a wrap is not virtualized); assertions run outside the
//! timed regions and check the lines against independent arithmetic, where input regions are
//! and how many callbacks ran.
use zaxis::testing::Inspect;
use zaxis::winit::event::{ElementState, MouseButton};
use zaxis::{Button, Context, InputEvent, Padding, Rect, Response, Root, TextEdit, Vec2, Wrap};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapCase {
    Row,
    Cached,
    Narrow,
    Wide,
    Resize,
    Change,
    Mixed,
    Compound,
    Click,
    Reorder,
    Lifecycle,
}
impl WrapCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Row => "wrap_plain_row",
            Self::Cached => "wrap_cached",
            Self::Narrow => "wrap_narrow",
            Self::Wide => "wrap_wide",
            Self::Resize => "wrap_resize",
            Self::Change => "wrap_change_one",
            Self::Mixed => "wrap_mixed_heights",
            Self::Compound => "wrap_compound",
            Self::Click => "wrap_click_item",
            Self::Reorder => "wrap_reorder",
            Self::Lifecycle => "wrap_mount_unmount",
        }
    }
    pub fn interactive(self) -> bool {
        matches!(self, Self::Resize | Self::Click)
    }
}

const GAP: Vec2 = Vec2::new(6.0, 6.0);
const PADDING: f32 = 8.0;
const WIDE: u32 = 1400;
const NARROW: u32 = 420;

/// What one pass built: the build-time responses, the wrap's report and the callbacks run.
#[derive(Default)]
struct Built {
    responses: Vec<Response>,
    lines: usize,
    rect: Rect,
    items: Vec<Rect>,
    callbacks: usize,
    /// The key of each item in declaration order, for ids that must not move with lines.
    keys: Vec<usize>,
}

pub struct Probe {
    kind: WrapCase,
    count: usize,
    step: usize,
    built: Built,
    mounted: bool,
    physical_width: u32,
    labels: Vec<String>,
    order: Vec<usize>,
    drafts: Vec<String>,
    selected: Vec<bool>,
    clicks: usize,
    before_clicks: usize,
    tessellations: u64,
    rebuilds: u64,
    ids: std::cell::RefCell<std::collections::HashMap<usize, zaxis::Id>>,
}

impl Probe {
    pub fn new(kind: WrapCase, count: usize) -> Self {
        let count = count.clamp(1, 5000);
        Self {
            kind,
            count,
            step: 0,
            built: Built::default(),
            mounted: true,
            physical_width: 1280,
            labels: (0..count).map(|i| format!("Tag {i}")).collect(),
            order: (0..count).collect(),
            drafts: vec![String::new(); count],
            selected: vec![false; count],
            clicks: 0,
            before_clicks: 0,
            tessellations: 0,
            rebuilds: 0,
            ids: Default::default(),
        }
    }

    fn pointer(c: &mut Context, p: Vec2) {
        let p = p * c.scale_factor();
        c.on_input(InputEvent::PointerMoved {
            x: p.x as f64,
            y: p.y as f64,
        });
    }

    fn button(c: &mut Context, state: ElementState) {
        c.on_input(InputEvent::Button {
            button: MouseButton::Left,
            state,
        });
    }

    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.before_clicks = self.clicks;
        self.tessellations = c.cache_stats().tessellated_elements;
        self.rebuilds = c.cache_stats().geometry_rebuilds;
        match self.kind {
            WrapCase::Resize => {
                self.physical_width = if step.is_multiple_of(2) { WIDE } else { NARROW };
                c.on_input(InputEvent::Resized {
                    width: self.physical_width,
                    height: 720,
                });
            }
            WrapCase::Change => {
                let i = self.count / 2;
                self.labels[i] = if step.is_multiple_of(2) {
                    format!("Tag {i}")
                } else {
                    format!("A noticeably longer tag {i}")
                };
            }
            WrapCase::Reorder => self.order.reverse(),
            WrapCase::Lifecycle => self.mounted = step.is_multiple_of(2),
            WrapCase::Click => {
                if let Some(last) = self.built.responses.last() {
                    let at = c.visual_rect(last.id, last.rect).center();
                    Self::pointer(c, at);
                    Self::button(c, ElementState::Pressed);
                    Self::button(c, ElementState::Released);
                }
            }
            _ => {}
        }
    }

    fn explicit_width(&self) -> Option<f32> {
        match self.kind {
            WrapCase::Narrow => Some(260.0),
            WrapCase::Wide => Some(1200.0),
            _ => None,
        }
    }

    pub fn build(&mut self, c: &mut Context) {
        let mut built = Built::default();
        let count = if self.mounted { self.count } else { 0 };
        let width = self.explicit_width();
        let Self {
            kind,
            labels,
            order,
            drafts,
            selected,
            clicks,
            ..
        } = self;
        c.run(|c| {
            Root::new().padding(Padding::all(PADDING)).show(c, |ui| {
                if *kind == WrapCase::Row {
                    ui.horizontal(|ui| {
                        for (i, label) in labels.iter().enumerate().take(count) {
                            built.callbacks += 1;
                            ui.push_id(i, |ui| built.responses.push(ui.button(label)));
                        }
                    });
                    return;
                }
                let mut wrap = Wrap::new("bench").spacing(GAP);
                if let Some(width) = width {
                    wrap = wrap.width(width);
                }
                let out = wrap.show(ui, |ui| {
                    for &i in order.iter().take(count) {
                        built.callbacks += 1;
                        let response = match kind {
                            WrapCase::Compound => ui.item(i, |ui| {
                                ui.add(TextEdit::new(&mut drafts[i]).width(90.0));
                                ui.button("Go")
                            }),
                            WrapCase::Mixed => ui.item(i, |ui| {
                                let text = "word ".repeat(1 + (i * 7) % 9);
                                ui.with_max_width(110.0, |ui| {
                                    ui.add(zaxis::Text::new(text).wrap(true))
                                })
                            }),
                            WrapCase::Reorder => ui.item(i, |ui| ui.button(&labels[i])),
                            _ => ui.push_id(i, |ui| {
                                let r = ui.add(
                                    Button::new(&labels[i])
                                        .id_source("tag")
                                        .selected(selected[i]),
                                );
                                if r.clicked() {
                                    selected[i] = !selected[i];
                                    *clicks += 1;
                                }
                                r
                            }),
                        };
                        built.keys.push(i);
                        built.responses.push(response);
                    }
                });
                built.lines = out.lines.len();
                built.rect = out.rect;
                built.items = out.items;
            });
        });
        self.built = built;
    }

    /// Lines of the same items by plain greedy arithmetic, independent of the component.
    fn expected_lines(&self, width: f32) -> usize {
        let mut lines = 0;
        let mut used = 0.0_f32;
        for response in &self.built.responses {
            let w = response.rect.size().x;
            if lines == 0 || used + GAP.x + w > width + 0.01 {
                lines += 1;
                used = w;
            } else {
                used += GAP.x + w;
            }
        }
        lines
    }

    pub fn verify(&self, c: &Context) {
        let b = &self.built;
        let expected = if self.mounted { self.count } else { 0 };
        assert_eq!(
            b.callbacks, expected,
            "an item callback ran more or less than once"
        );
        assert_eq!(b.responses.len(), expected);
        let probe = c.probe();
        assert!(
            probe.previous_hits.len() <= 12 * expected + 32,
            "{} hit regions for {expected} items",
            probe.previous_hits.len()
        );
        assert!(probe.input_transforms.len() <= 16 * expected + 64);
        assert!(
            probe.cache.len() <= 8 * expected + 64,
            "{}",
            probe.cache.len()
        );
        if !self.mounted {
            assert!(
                probe.previous_hits.len() <= 4,
                "unmounted items left regions"
            );
            return;
        }
        if self.kind == WrapCase::Row {
            let xs: Vec<f32> = b.responses.iter().map(|r| r.rect.min.x).collect();
            assert!(
                xs.windows(2).all(|w| w[0] < w[1]),
                "a plain row is one line"
            );
            return;
        }
        let area = c.viewport().size().x - 2.0 * PADDING;
        let width = self.explicit_width().unwrap_or(area);
        assert_eq!(b.items.len(), expected);
        if self.kind != WrapCase::Compound && self.kind != WrapCase::Mixed {
            assert_eq!(
                b.lines,
                self.expected_lines(width),
                "lines at width {width}"
            );
        }
        assert!(b.rect.size().x >= width - 0.01 && b.rect.is_finite());
        // Where an item is shown, where its input region is and where the wrap reports it
        // agree: first, last and a stride of the rest.
        let stride = (expected / 24).max(1);
        for i in (0..expected).step_by(stride).chain([expected - 1]) {
            let (item, r) = (b.items[i], &b.responses[i]);
            if item.min.y >= c.viewport().max.y || item.max.y <= 0.0 {
                // Outside the window nothing is published for it; the wrap still reports it.
                assert!(!probe.previous_hits.iter().any(|h| h.id == r.id
                    && !h.clip.is_empty()
                    && h.rect.intersect(h.clip).size().y > 0.0
                    && h.rect.min.y >= c.viewport().max.y));
                continue;
            }
            let shown = c.visual_rect(r.id, r.rect);
            assert!(
                shown.min.x >= item.min.x - 1.0
                    && shown.min.y >= item.min.y - 1.0
                    && shown.max.x <= item.max.x + 1.0
                    && shown.max.y <= item.max.y + 1.0,
                "item {i} is shown at {shown:?}, outside {item:?}"
            );
            if let Some(hit) = probe.previous_hits.iter().find(|h| h.id == r.id) {
                assert!(
                    (hit.rect.min - shown.min).abs().max_element() < 0.01,
                    "item {i}: region {:?} is not where it is shown {shown:?}",
                    hit.rect
                );
            }
            assert!(item.max.x <= PADDING + width.max(item.size().x) + 0.01);
        }
        // An item keeps its id whichever line or position it is on.
        let mut ids = self.ids.borrow_mut();
        for (key, r) in b.keys.iter().zip(&b.responses) {
            assert_eq!(
                *ids.entry(*key).or_insert(r.id),
                r.id,
                "item {key} changed its id"
            );
        }
        drop(ids);
        match self.kind {
            WrapCase::Mixed => {
                let first = b.items[0].size().y;
                assert!(
                    expected < 3 || b.items.iter().any(|r| (r.size().y - first).abs() > 1.0),
                    "mixed heights are all equal"
                );
            }
            WrapCase::Reorder if self.step > 12 => {
                let key = if self.step.is_multiple_of(2) {
                    0
                } else {
                    expected - 1
                };
                assert!(self.order.len() == expected && b.keys[0] == self.order[0]);
                assert!(key < expected);
            }
            WrapCase::Cached | WrapCase::Narrow | WrapCase::Wide if self.step > 12 => {
                assert!(!c.needs_repaint(), "an idle wrap keeps asking for passes");
                assert_eq!(self.tessellations, c.cache_stats().tessellated_elements);
                assert_eq!(self.rebuilds, c.cache_stats().geometry_rebuilds);
            }
            WrapCase::Resize if self.step > 2 => {
                let wide = self.physical_width == WIDE;
                let last = b.items.last().expect("items").min.y;
                assert_eq!(b.lines > 1, !wide || expected > 60, "lines {}", b.lines);
                assert!(last >= 0.0);
            }
            WrapCase::Click if self.step > 12 => {
                assert_eq!(
                    self.clicks,
                    self.before_clicks + 1,
                    "the last item took {} clicks",
                    self.clicks - self.before_clicks
                );
            }
            _ => {}
        }
    }
}
