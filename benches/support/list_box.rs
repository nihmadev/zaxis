//! Real `ListBox` paths: cached idle, keyboard scrolling, selection, model revisions,
//! measured rows and lifecycle. Assertions run outside the timed regions.
use std::{collections::HashSet, time::Duration};
use zaxis::winit::{event::ElementState, keyboard::KeyCode};
use zaxis::Instant;
use zaxis::{Context, Id, ListBox, ListEntry, ListMode, ListOutput, Root};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListCase {
    Cached,
    Scroll,
    Keys,
    Select,
    Revision,
    Measured,
    Lifecycle,
}
impl ListCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::Cached => "list_cached",
            Self::Scroll => "list_scroll_keys",
            Self::Keys => "list_arrow_keys",
            Self::Select => "list_select",
            Self::Revision => "list_revision",
            Self::Measured => "list_measured_scroll",
            Self::Lifecycle => "list_lifecycle",
        }
    }
    pub fn interactive(self) -> bool {
        !matches!(self, Self::Cached | Self::Lifecycle)
    }
}

pub struct Probe {
    kind: ListCase,
    keys: Vec<u32>,
    names: Vec<String>,
    selected: HashSet<Id>,
    out: Option<ListOutput>,
    now: Instant,
    step: usize,
    revision: u64,
    rows: usize,
    draw_revision: u64,
    tessellations: u64,
}
impl Probe {
    pub fn new(kind: ListCase, count: usize) -> Self {
        let count = count.max(2) as u32;
        Self {
            kind,
            keys: (0..count).collect(),
            names: (0..count).map(|i| format!("Entry {i}")).collect(),
            selected: HashSet::new(),
            out: None,
            now: Instant::now(),
            step: 0,
            revision: 0,
            rows: 0,
            draw_revision: 0,
            tessellations: 0,
        }
    }
    fn press(c: &mut Context, key: KeyCode) {
        c.on_key_event(key, ElementState::Pressed, false);
        c.on_key_event(key, ElementState::Released, false);
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.draw_revision = c.draw_data().revision;
        self.tessellations = c.cache_stats().tessellated_elements;
        if let Some(out) = &self.out {
            c.request_focus(out.id);
        }
        match self.kind {
            ListCase::Scroll | ListCase::Measured => Self::press(
                c,
                if step.is_multiple_of(2) {
                    KeyCode::End
                } else {
                    KeyCode::Home
                },
            ),
            ListCase::Keys => Self::press(c, KeyCode::ArrowDown),
            ListCase::Select => Self::press(
                c,
                if step.is_multiple_of(2) {
                    KeyCode::ArrowDown
                } else {
                    KeyCode::Space
                },
            ),
            ListCase::Revision => {
                self.keys.rotate_left(1);
                self.revision += 1;
            }
            ListCase::Cached | ListCase::Lifecycle => {}
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        self.rows = 0;
        if self.kind == ListCase::Lifecycle && self.step % 2 == 1 {
            c.run_at(self.now, |_| {});
            self.out = None;
            return;
        }
        let (keys, names) = (&self.keys, &self.names);
        let (selected, rows) = (&mut self.selected, &mut self.rows);
        let mut list = ListBox::new("bench")
            .mode(ListMode::Multiple)
            .selection(selected)
            .max_height(200.0)
            .revision(self.revision);
        list = if self.kind == ListCase::Measured {
            list.measured_rows(24.0)
        } else {
            list.row_height(24.0)
        };
        let measured = self.kind == ListCase::Measured;
        let mut out = None;
        c.run_at(self.now, |c| {
            Root::new().show(c, |ui| {
                out = Some(list.show_rows(
                    ui,
                    keys.len(),
                    |i| ListEntry::item(keys[i], &names[keys[i] as usize]),
                    |ui, row| {
                        *rows += 1;
                        ui.label(row.text);
                        if measured && row.index % 3 == 0 {
                            ui.label("second line");
                        }
                    },
                ));
            })
        });
        self.out = out;
    }
    pub fn verify(&self, c: &Context) {
        let Some(out) = &self.out else {
            assert!(!c.wants_animation_frame());
            return;
        };
        assert_eq!(out.len, self.keys.len());
        assert!(out.rows_built <= 16, "built {} rows", out.rows_built);
        assert_eq!(out.rows_built, self.rows);
        match self.kind {
            ListCase::Cached if self.step > 12 => {
                assert!(!out.rebuilt && !c.needs_repaint());
                assert_eq!(self.tessellations, c.cache_stats().tessellated_elements);
                assert_eq!(self.draw_revision, c.draw_data().revision);
            }
            ListCase::Scroll | ListCase::Measured if self.step > 12 => {
                let last = self.keys.len() - 1;
                if self.step.is_multiple_of(2) {
                    assert!(out.visible.contains(&last) || self.kind == ListCase::Measured);
                    assert!(out.scroll_offset.y > 0.0 || self.keys.len() < 10);
                } else {
                    assert_eq!(out.visible.start, 0);
                }
            }
            ListCase::Keys if self.step > 12 => assert!(out.active.is_some()),
            ListCase::Select if self.step > 12 => assert!(!self.selected.is_empty()),
            ListCase::Revision if self.step > 12 => assert!(out.rebuilt),
            _ => {}
        }
    }
}
