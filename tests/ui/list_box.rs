use crate::prelude::*;
use std::{collections::HashSet, time::Duration};
use winit::{
    dpi::PhysicalSize,
    event::ElementState,
    keyboard::{KeyCode, ModifiersState},
};
use zaxis::Instant;
use zaxis::{Id, ListBox, ListEntry, ListEvent, ListMode, ListOutput, Root};

mod measured;
mod model;

pub(crate) const ROW: f32 = 24.0;

#[derive(Clone)]
pub(crate) struct Item {
    pub key: u32,
    pub text: String,
    pub enabled: bool,
    pub header: bool,
    pub lines: usize,
}
pub(crate) fn item(key: u32, text: &str) -> Item {
    Item {
        key,
        text: text.into(),
        enabled: true,
        header: false,
        lines: 1,
    }
}
pub(crate) fn numbered(count: u32) -> Vec<Item> {
    (0..count).map(|i| item(i, &format!("Row {i}"))).collect()
}
pub(crate) fn k(key: u32) -> Id {
    Id::new(key)
}

pub(crate) struct List {
    pub c: Context,
    pub items: Vec<Item>,
    pub sel: HashSet<Id>,
    pub mode: ListMode,
    pub out: Option<ListOutput>,
    pub events: Vec<ListEvent>,
    pub revision: u64,
    pub measured: Option<f32>,
    pub drag: bool,
    pub height: f32,
    pub buttons: bool,
    pub scroll_to: Option<u32>,
    pub more: bool,
    pub step: u64,
    start: Instant,
    pub clicked_button: u32,
    flip: bool,
}
impl List {
    pub fn with_scale(items: Vec<Item>, mode: ListMode, scale: f64) -> Self {
        let mut c = Context::new();
        c.set_viewport(
            PhysicalSize::new((800.0 * scale) as u32, (600.0 * scale) as u32),
            scale,
        );
        let mut style = c.style().clone();
        style.motion.reduced_motion = true;
        c.set_style(style);
        let mut list = Self {
            c,
            items,
            sel: HashSet::new(),
            mode,
            out: None,
            events: Vec::new(),
            revision: 0,
            measured: None,
            drag: false,
            height: 240.0,
            buttons: false,
            scroll_to: None,
            more: false,
            step: 0,
            start: Instant::now(),
            clicked_button: 0,
            flip: false,
        };
        list.frame();
        list.frame();
        list
    }
    pub fn new(items: Vec<Item>, mode: ListMode) -> Self {
        Self::with_scale(items, mode, 1.0)
    }
    pub fn frame(&mut self) -> &ListOutput {
        self.step += 1;
        let now = self.start + Duration::from_millis(self.step * 16);
        let (items, sel) = (&self.items, &mut self.sel);
        let (buttons, scroll_to, clicked) = (
            self.buttons,
            self.scroll_to.take(),
            &mut self.clicked_button,
        );
        let mut out = None;
        let mut builder = ListBox::new("list")
            .mode(self.mode)
            .selection(sel)
            .row_height(ROW)
            .max_height(self.height)
            .revision(self.revision)
            .drag_rows(self.drag)
            .has_more(self.more);
        if let Some(estimate) = self.measured {
            builder = builder.measured_rows(estimate);
        }
        if let Some(key) = scroll_to {
            builder = builder.scroll_to_key(key);
        }
        self.c.run_at(now, |c| {
            Root::new().show(c, |ui| {
                out = Some(builder.show_slice(
                    ui,
                    items,
                    |it| {
                        let e = if it.header {
                            ListEntry::header(it.key, &it.text)
                        } else {
                            ListEntry::item(it.key, &it.text)
                        };
                        e.enabled(it.enabled)
                    },
                    |ui, row, it| {
                        ui.vertical(|ui| {
                            for line in 0..it.lines {
                                ui.label(format!("{} {line}", it.text));
                            }
                        });
                        if buttons {
                            row.trailing(ui, |ui| {
                                if ui.button("Go").clicked() {
                                    *clicked += 1;
                                }
                            });
                        }
                    },
                ));
            });
        });
        let out = out.unwrap();
        self.events.extend(out.events.iter().copied());
        self.out.insert(out)
    }
    pub fn out(&self) -> &ListOutput {
        self.out.as_ref().unwrap()
    }
    pub fn take_events(&mut self) -> Vec<ListEvent> {
        std::mem::take(&mut self.events)
    }
    /// Center of the row at `index` (fixed-height lists), in the list's own scroll position.
    pub fn at(&self, index: usize) -> Vec2 {
        let o = self.out();
        Vec2::new(
            o.viewport.min.x + 40.0,
            o.viewport.min.y + (index as f32 + 0.5) * ROW - o.scroll_offset.y,
        )
    }
    pub fn click(&mut self, index: usize) {
        self.click_with(index, ModifiersState::empty());
    }
    pub fn click_with(&mut self, index: usize, mods: ModifiersState) {
        // Successive clicks land apart, so they never count as a double click.
        self.flip = !self.flip;
        let p = self.at(index) + Vec2::new(if self.flip { 30.0 } else { 0.0 }, 0.0);
        self.click_at(p, mods);
    }
    pub fn click_at(&mut self, p: Vec2, mods: ModifiersState) {
        self.c.set_modifiers(mods);
        self.c.move_pointer(p);
        self.c.primary_button(ElementState::Pressed);
        self.c.primary_button(ElementState::Released);
        self.frame();
        self.c.set_modifiers(ModifiersState::empty());
    }
    pub fn key_with(&mut self, code: KeyCode, mods: ModifiersState) {
        self.c.set_modifiers(mods);
        self.c.on_key_event(code, ElementState::Pressed, false);
        self.c.on_key_event(code, ElementState::Released, false);
        self.frame();
        self.c.set_modifiers(ModifiersState::empty());
    }
    pub fn key(&mut self, code: KeyCode) {
        self.key_with(code, ModifiersState::empty());
    }
    pub fn type_text(&mut self, text: &str) {
        self.c.on_text_event(text);
        self.frame();
    }
    pub fn wait(&mut self, millis: u64) {
        self.step += millis / 16;
    }
    pub fn selected(&self) -> Vec<u32> {
        let mut keys: Vec<u32> = self
            .items
            .iter()
            .filter(|it| self.sel.contains(&k(it.key)))
            .map(|it| it.key)
            .collect();
        keys.sort_unstable();
        keys
    }
    pub fn active(&self) -> Option<u32> {
        let active = self.out().active?;
        self.items
            .iter()
            .find(|it| k(it.key) == active)
            .map(|it| it.key)
    }
    pub fn settled(&self) -> bool {
        !self
            .c
            .needs_repaint_at(self.c.frame_time() + Duration::from_secs(10))
    }
}

const CTRL: ModifiersState = ModifiersState::CONTROL;
const SHIFT: ModifiersState = ModifiersState::SHIFT;

#[test]
fn single_click_selects_once_and_repeat_changes_nothing() {
    let mut l = List::new(numbered(20), ListMode::Single);
    l.click(3);
    assert_eq!(l.take_events(), vec![ListEvent::SelectionChanged]);
    assert_eq!(l.selected(), vec![3]);
    assert!(l.frame().events.is_empty(), "one input, one event");
    l.click(3);
    assert!(l.take_events().is_empty());
    l.click(5);
    assert_eq!(l.selected(), vec![5]);
}

#[test]
fn multiple_selection_with_modifiers() {
    let mut l = List::new(numbered(30), ListMode::Multiple);
    l.click(2);
    l.click_with(4, CTRL);
    assert_eq!(l.selected(), vec![2, 4]);
    l.click_with(4, CTRL);
    assert_eq!(l.selected(), vec![2]);
    l.click_with(6, SHIFT);
    assert_eq!(
        l.selected(),
        vec![4, 5, 6],
        "the range runs from the last clicked row"
    );
    l.click_with(1, SHIFT);
    assert_eq!(
        l.selected(),
        vec![1, 2, 3, 4],
        "the anchor stays, the range follows"
    );
    l.click_with(8, CTRL);
    l.click_with(9, ModifiersState::CONTROL | ModifiersState::SHIFT);
    assert_eq!(
        l.selected(),
        vec![1, 2, 3, 4, 8, 9],
        "Ctrl+Shift adds the range"
    );
    l.click(5);
    assert_eq!(l.selected(), vec![5]);
    l.take_events();
    l.key_with(KeyCode::KeyA, CTRL);
    assert_eq!(l.selected().len(), 30);
    assert_eq!(l.take_events(), vec![ListEvent::SelectionChanged]);
    l.key(KeyCode::Escape);
    assert!(l.selected().is_empty());
    assert_eq!(l.take_events(), vec![ListEvent::SelectionChanged]);
}

#[test]
fn checks_mode_toggles_on_plain_click() {
    let mut l = List::new(numbered(10), ListMode::Checks);
    l.click(1);
    l.click(3);
    assert_eq!(l.selected(), vec![1, 3]);
    l.click(1);
    assert_eq!(l.selected(), vec![3]);
    l.key(KeyCode::ArrowDown);
    assert_eq!(l.selected(), vec![3], "arrows only move in Checks mode");
    l.key(KeyCode::Space);
    assert_eq!(l.selected(), vec![2, 3], "Space toggles the active row");
}

#[test]
fn double_click_and_enter_activate_and_context_reports() {
    let mut l = List::new(numbered(10), ListMode::Single);
    let p = l.at(2);
    l.c.move_pointer(p);
    for _ in 0..2 {
        l.c.primary_button(ElementState::Pressed);
        l.c.primary_button(ElementState::Released);
    }
    l.frame();
    let events = l.take_events();
    assert!(events.contains(&ListEvent::Activated(k(2))), "{events:?}");
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, ListEvent::Activated(_)))
            .count(),
        1
    );
    l.click(4);
    l.take_events();
    l.key(KeyCode::Enter);
    assert_eq!(l.take_events(), vec![ListEvent::Activated(k(4))]);
    l.c.move_pointer(l.at(6));
    l.c.secondary_button(ElementState::Pressed);
    l.c.secondary_button(ElementState::Released);
    l.frame();
    assert_eq!(l.take_events(), vec![ListEvent::Context(k(6))]);
    assert_eq!(
        l.selected(),
        vec![4],
        "the application decides about selecting"
    );
}

#[test]
fn keyboard_navigation_skips_disabled_and_headers() {
    let mut items = numbered(40);
    items[3].enabled = false;
    items[4].header = true;
    let mut l = List::new(items, ListMode::Single);
    l.click(2);
    l.key(KeyCode::ArrowDown);
    assert_eq!(
        l.active(),
        Some(5),
        "rows 3 (disabled) and 4 (header) are skipped"
    );
    assert_eq!(l.selected(), vec![5]);
    l.key(KeyCode::ArrowUp);
    assert_eq!(l.active(), Some(2));
    l.key(KeyCode::End);
    assert_eq!(l.active(), Some(39));
    l.key(KeyCode::Home);
    assert_eq!(l.active(), Some(0));
    l.key(KeyCode::PageDown);
    let page = l.active().unwrap();
    assert!(
        (8..=12).contains(&page),
        "a page is about a viewport: {page}"
    );
    l.key(KeyCode::PageUp);
    assert_eq!(l.active(), Some(0));
    assert!(l.out().scroll_offset.y <= 0.5);
    l.key(KeyCode::End);
    assert!(l.out().scroll_offset.y > 0.0, "the active row is revealed");
    let o = l.out();
    assert!(o.visible.contains(&39));
}

#[test]
fn ctrl_arrows_move_without_selecting_and_space_toggles() {
    let mut l = List::new(numbered(10), ListMode::Multiple);
    l.click(1);
    l.key_with(KeyCode::ArrowDown, CTRL);
    assert_eq!(l.active(), Some(2));
    assert_eq!(l.selected(), vec![1]);
    l.key(KeyCode::Space);
    assert_eq!(l.selected(), vec![1, 2]);
    l.key_with(KeyCode::ArrowDown, SHIFT);
    l.key_with(KeyCode::ArrowDown, SHIFT);
    assert_eq!(
        l.selected(),
        vec![2, 3, 4],
        "Shift+arrows select from the anchor"
    );
}

#[test]
fn type_ahead_accumulates_cycles_and_times_out() {
    let names = ["Apple", "Banana", "Blueberry", "Cherry", "Bamboo"];
    let mut l = List::new(
        names
            .iter()
            .enumerate()
            .map(|(i, n)| item(i as u32, n))
            .collect(),
        ListMode::Single,
    );
    l.click(0);
    l.type_text("b");
    assert_eq!(l.active(), Some(1));
    l.type_text("l");
    assert_eq!(l.active(), Some(2), "\"bl\" extends the search");
    l.wait(1000);
    l.type_text("b");
    assert_eq!(
        l.active(),
        Some(4),
        "after the timeout a new search starts after the active row"
    );
    l.wait(1000);
    l.type_text("c");
    assert_eq!(l.active(), Some(3));
    l.wait(100);
    l.type_text("c");
    assert_eq!(l.active(), Some(3), "cc keeps cycling within matches");
}

#[test]
fn type_ahead_respects_grapheme_boundaries_and_case() {
    let items = vec![item(0, "x"), item(1, "e\u{301}clair"), item(2, "Eagle")];
    let mut l = List::new(items, ListMode::Single);
    l.click(0);
    l.type_text("e");
    assert_eq!(l.active(), Some(2), "a decomposed é is not an e");
    l.wait(1000);
    l.type_text("E\u{301}");
    assert_eq!(l.active(), Some(1));
}

#[test]
fn disabled_rows_ignore_the_pointer_and_the_keyboard() {
    let mut items = numbered(10);
    items[2].enabled = false;
    let mut l = List::new(items, ListMode::Multiple);
    l.click(2);
    assert!(l.selected().is_empty());
    l.click(0);
    l.key_with(KeyCode::KeyA, CTRL);
    assert!(!l.selected().contains(&2), "select-all skips disabled rows");
    assert_eq!(l.selected().len(), 9);
}

#[test]
fn nested_button_works_without_selecting_the_row() {
    let mut l = List::new(numbered(10), ListMode::Single);
    l.buttons = true;
    l.frame();
    let o = l.out().viewport;
    l.click_at(
        Vec2::new(o.max.x - 40.0, l.at(2).y),
        ModifiersState::empty(),
    );
    assert_eq!(l.clicked_button, 1);
    assert!(l.selected().is_empty(), "the button took the click");
    l.click(2);
    assert_eq!(l.selected(), vec![2]);
    assert_eq!(l.clicked_button, 1);
}

#[test]
fn hover_does_not_move_the_keyboard_row() {
    let mut l = List::new(numbered(10), ListMode::Single);
    l.click(1);
    l.c.move_pointer(l.at(6));
    l.frame();
    l.frame();
    assert_eq!(l.active(), Some(1));
    l.key(KeyCode::ArrowDown);
    assert_eq!(
        l.active(),
        Some(2),
        "movement continues from the keyboard row"
    );
}

#[test]
fn keys_go_to_the_list_only_while_it_has_focus() {
    let mut l = List::new(numbered(10), ListMode::Single);
    l.key(KeyCode::ArrowDown);
    assert!(l.selected().is_empty());
    l.click(0);
    l.key(KeyCode::ArrowDown);
    assert_eq!(l.selected(), vec![1]);
}
