//! The accessibility tree on real widgets: the cost of the layer with nobody listening,
//! an idle tree, one changed node, a whole tree, and a text field being typed into.
//! `count` is the number of rows; a row is a label, a button and a checkbox, so the tree
//! has about three nodes per row plus one text run per label. Assertions run outside the
//! timed regions.
use std::time::Duration;
use zaxis::Instant;
use zaxis::{Context, Response, Root, TextEdit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessCase {
    /// The scene of the other cases with no assistive technology connected: the baseline.
    IdleOff,
    IdleOn,
    ChangeOne,
    FullTree,
    Text,
}
impl AccessCase {
    pub fn name(self) -> &'static str {
        match self {
            Self::IdleOff => "access_off_idle",
            Self::IdleOn => "access_on_idle",
            Self::ChangeOne => "access_change_one",
            Self::FullTree => "access_full_tree",
            Self::Text => "access_text_typing",
        }
    }
    pub fn interactive(self) -> bool {
        matches!(self, Self::ChangeOne | Self::Text)
    }
}

#[cfg(feature = "accesskit")]
fn take(c: &mut Context) -> Option<(usize, bool)> {
    c.take_accessibility_update()
        .map(|update| (update.nodes.len(), update.tree.is_some()))
}
#[cfg(not(feature = "accesskit"))]
fn take(_: &mut Context) -> Option<(usize, bool)> {
    None
}

pub struct Probe {
    kind: AccessCase,
    labels: Vec<String>,
    checks: Vec<bool>,
    text: String,
    field: Option<Response>,
    now: Instant,
    step: usize,
    /// Nodes and completeness of the update the last pass published.
    update: Option<(usize, bool)>,
    draw_revision: u64,
}
impl Probe {
    pub fn new(c: &mut Context, kind: AccessCase, count: usize) -> Self {
        c.set_accessibility_active(kind != AccessCase::IdleOff);
        Self {
            kind,
            labels: (0..count).map(|i| format!("Row {i}")).collect(),
            checks: vec![false; count],
            // Ten characters per object: `--sizes 1000` is a 10 000 character document.
            text: (0..count).map(|i| format!("line {i:04}\n")).collect(),
            field: None,
            now: Instant::now(),
            step: 0,
            update: None,
            draw_revision: 0,
        }
    }
    pub fn input(&mut self, c: &mut Context, step: usize) {
        self.step = step;
        self.now += Duration::from_millis(20);
        self.draw_revision = c.draw_data().revision;
        match self.kind {
            AccessCase::ChangeOne => {
                let row = step % self.checks.len().max(1);
                if let Some(check) = self.checks.get_mut(row) {
                    *check = !*check;
                }
            }
            AccessCase::FullTree => {
                // Assistive technology reconnecting: the next pass sends everything.
                c.set_accessibility_active(false);
                c.set_accessibility_active(true);
            }
            AccessCase::Text => {
                if let Some(field) = self.field {
                    c.request_focus(field.id);
                }
                c.on_text_event("x");
            }
            AccessCase::IdleOff | AccessCase::IdleOn => {}
        }
    }
    pub fn build(&mut self, c: &mut Context) {
        let (labels, checks, text) = (&self.labels, &mut self.checks, &mut self.text);
        let mut field = None;
        let typing = self.kind == AccessCase::Text;
        c.run_at(self.now, |c| {
            Root::new().show(c, |ui| {
                if typing {
                    field = Some(ui.add(TextEdit::new(text).multiline().rows(20.0)));
                    return;
                }
                for (label, check) in labels.iter().zip(checks.iter_mut()) {
                    ui.horizontal(|ui| {
                        ui.label(label.as_str());
                        ui.push_id(label, |ui| {
                            ui.button("Open");
                            ui.checkbox(check, "Pin");
                        });
                    });
                }
            })
        });
        self.field = field;
        self.update = take(c);
    }
    pub fn verify(&self, c: &Context) {
        if self.step <= 12 {
            return;
        }
        #[cfg(feature = "accesskit")]
        {
            let stats = c.accessibility_stats();
            let rows = self.labels.len();
            match self.kind {
                AccessCase::IdleOff => {
                    assert_eq!(
                        stats.passes, 0,
                        "nothing is collected with nobody listening"
                    );
                    assert!(self.update.is_none());
                }
                AccessCase::IdleOn => {
                    assert!(self.update.is_none(), "an idle tree publishes nothing");
                    assert_eq!(stats.nodes as usize, rows * 3 + 1);
                    assert_eq!(self.draw_revision, c.draw_data().revision);
                }
                AccessCase::ChangeOne => {
                    assert_eq!(self.update, Some((1, false)), "one checkbox, one node");
                }
                AccessCase::FullTree => {
                    let (nodes, full) = self.update.expect("a full tree");
                    // Rows, their label runs, the root layer and the window.
                    assert!(
                        full && nodes == rows * 4 + 2,
                        "{nodes} nodes for {rows} rows"
                    );
                }
                AccessCase::Text => {
                    let (nodes, full) = self.update.expect("typing changes the field");
                    assert!(!full && (2..64).contains(&nodes), "{nodes} nodes");
                    assert!(self.text.ends_with('x'), "typed at the caret, the end");
                }
            }
        }
        #[cfg(not(feature = "accesskit"))]
        let _ = c;
    }
}
