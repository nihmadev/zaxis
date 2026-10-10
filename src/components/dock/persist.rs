//! A versioned, bounded whitespace format; no serde dependency or application payloads.
use super::{DockChild, DockError, DockFloat, DockIssue, DockNode, DockState, PanelId};
use crate::{Id, Layout, Rect, Vec2};
use std::{collections::HashSet, fmt::Write};

impl DockState {
    /// Save IDs as exact hexadecimal values. Use stable application values across launches.
    pub fn save(&self) -> String {
        fn copy(node: &DockNode, depth: usize) -> Option<DockNode> {
            if depth > 128 {
                return None;
            }
            Some(match node {
                DockNode::Tabs { id, panels, active } => DockNode::Tabs {
                    id: *id,
                    panels: panels.clone(),
                    active: *active,
                },
                DockNode::Split { id, axis, children } => DockNode::Split {
                    id: *id,
                    axis: *axis,
                    children: children
                        .iter()
                        .filter_map(|c| {
                            copy(&c.node, depth + 1).map(|node| DockChild::new(node, c.fraction))
                        })
                        .collect(),
                },
            })
        }
        let mut state = Self {
            root: self.root.as_ref().and_then(|r| copy(r, 0)),
            focused: self.focused,
            floats: self
                .floats
                .iter()
                .filter_map(|f| {
                    copy(&f.node, 0).map(|node| DockFloat {
                        node,
                        bounds: f.bounds,
                    })
                })
                .collect(),
        };
        state.normalize();
        let mut text = String::from("zaxis-dock 1 ");
        write_id(&mut text, state.focused);
        write_node(&mut text, state.root.as_ref());
        let _ = write!(text, "{} ", state.floats.len());
        for f in state.floats {
            let _ = write!(
                text,
                "{} {} {} {} ",
                f.bounds.min.x,
                f.bounds.min.y,
                f.bounds.size().x,
                f.bounds.size().y
            );
            write_node(&mut text, Some(&f.node));
        }
        text.push('\n');
        text
    }
    /// Restore available panels, skip removed IDs, repair active IDs and normalize.
    /// Malformed/excessively large data returns an error without modifying a live Dock.
    pub fn load(
        text: &str,
        available: impl IntoIterator<Item = PanelId>,
    ) -> Result<Self, DockError> {
        if text.len() > 4 * 1024 * 1024 {
            return Err(bad());
        }
        let mut reader = Reader {
            tokens: text.split_whitespace(),
            remaining: 16384,
        };
        if reader.word()? != "zaxis-dock" || reader.word()? != "1" {
            return Err(bad());
        }
        let focused = reader.id()?;
        let root = reader.node(0)?;
        let count = reader.count()?;
        let mut floats = Vec::new();
        for _ in 0..count {
            let min = Vec2::new(reader.number()?, reader.number()?);
            let size = Vec2::new(reader.number()?, reader.number()?);
            if let Some(node) = reader.node(0)? {
                floats.push(DockFloat {
                    node,
                    bounds: Rect::from_min_size(min, size),
                });
            }
        }
        if reader.tokens.next().is_some() {
            return Err(bad());
        }
        let mut state = Self {
            root,
            floats,
            focused,
        };
        let available: HashSet<_> = available.into_iter().collect();
        state.retain_panels(|p| available.contains(&p));
        Ok(state)
    }
}
fn write_id(text: &mut String, id: Option<Id>) {
    if let Some(id) = id {
        let _ = write!(text, "{:x} ", id.value());
    } else {
        text.push_str("- ");
    }
}
fn write_node(text: &mut String, node: Option<&DockNode>) {
    match node {
        None => text.push_str("E "),
        Some(DockNode::Tabs { id, panels, active }) => {
            text.push_str("T ");
            write_id(text, Some(*id));
            write_id(text, *active);
            let _ = write!(text, "{} ", panels.len());
            for p in panels {
                write_id(text, Some(*p));
            }
        }
        Some(DockNode::Split { id, axis, children }) => {
            text.push_str(if *axis == Layout::Horizontal {
                "H "
            } else {
                "V "
            });
            write_id(text, Some(*id));
            let _ = write!(text, "{} ", children.len());
            for c in children {
                let _ = write!(text, "{} ", c.fraction);
                write_node(text, Some(&c.node));
            }
        }
    }
}
struct Reader<'a> {
    tokens: std::str::SplitWhitespace<'a>,
    remaining: usize,
}
impl<'a> Reader<'a> {
    fn word(&mut self) -> Result<&'a str, DockError> {
        self.tokens.next().ok_or_else(bad)
    }
    fn id(&mut self) -> Result<Option<Id>, DockError> {
        let word = self.word()?;
        if word == "-" {
            return Ok(None);
        }
        u64::from_str_radix(word, 16)
            .map(|n| Some(Id::from_raw(n)))
            .map_err(|_| bad())
    }
    fn number(&mut self) -> Result<f32, DockError> {
        let n: f32 = self.word()?.parse().map_err(|_| bad())?;
        if !n.is_finite() {
            return Err(bad());
        }
        Ok(n)
    }
    fn count(&mut self) -> Result<usize, DockError> {
        let n = self.word()?.parse().map_err(|_| bad())?;
        if n > self.remaining {
            return Err(bad());
        }
        self.remaining -= n;
        Ok(n)
    }
    fn node(&mut self, depth: usize) -> Result<Option<DockNode>, DockError> {
        if depth > 128 {
            return Err(DockError(DockIssue::TooDeep));
        }
        let kind = self.word()?;
        if kind == "E" {
            return Ok(None);
        }
        let id = self.id()?.ok_or_else(bad)?;
        match kind {
            "T" => {
                let active = self.id()?;
                let n = self.count()?;
                let mut panels = Vec::new();
                for _ in 0..n {
                    panels.push(self.id()?.ok_or_else(bad)?);
                }
                Ok(Some(DockNode::Tabs { id, panels, active }))
            }
            "H" | "V" => {
                let n = self.count()?;
                let mut children = Vec::new();
                for _ in 0..n {
                    let fraction = self.number()?;
                    if let Some(node) = self.node(depth + 1)? {
                        children.push(DockChild::new(node, fraction));
                    }
                }
                Ok(Some(DockNode::Split {
                    id,
                    axis: if kind == "H" {
                        Layout::Horizontal
                    } else {
                        Layout::Vertical
                    },
                    children,
                }))
            }
            _ => Err(bad()),
        }
    }
}
fn bad() -> DockError {
    DockError(DockIssue::InvalidFormat)
}
