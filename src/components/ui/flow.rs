//! Single-pass scopes: measure each item once, then place paint and input together.
use super::*;
use crate::{context::placement::Placement, Align};

#[derive(Clone, Copy, PartialEq)]
struct Measure {
    size: Vec2,
    flexible: bool,
    gap: bool,
}

pub(crate) struct FlowState {
    pub(crate) last_frame: u64,
    measured: Vec<Measure>,
}

struct Item {
    measure: Measure,
    origin: Vec2,
    placement: Option<Placement>,
}

pub(crate) struct Flow {
    align: Align,
    items: Vec<Item>,
    previous: Vec<Measure>,
    pub(super) pending: Option<Rect>,
}

impl Flow {
    pub(super) fn space(&mut self, direction: Layout, amount: f32) {
        self.items.push(Item {
            measure: Measure {
                size: direction.size(amount.max(0.0), 0.0),
                flexible: false,
                gap: true,
            },
            origin: Vec2::ZERO,
            placement: None,
        });
    }
}

fn resolve(measured: &[Measure], direction: Layout, available: f32, spacing: f32) -> Vec<f32> {
    let count = measured.iter().filter(|m| m.flexible).count();
    let fixed: f32 = measured
        .iter()
        .filter(|m| !m.flexible)
        .map(|m| direction.main(m.size))
        .sum();
    let gaps = spacing * measured.iter().filter(|m| !m.gap).count().saturating_sub(1) as f32;
    let share = if count == 0 {
        0.0
    } else {
        (available - fixed - gaps).max(0.0) / count as f32
    };
    measured
        .iter()
        .map(|m| {
            if m.flexible {
                share
            } else {
                direction.main(m.size)
            }
        })
        .collect()
}

impl Ui<'_> {
    pub(crate) fn begin_layout(&mut self, align: Align) {
        let previous = self
            .context
            .layouts
            .remove(&self.scope)
            .map_or(Vec::new(), |s| s.measured);
        self.flow = Some(Flow {
            align,
            items: Vec::new(),
            previous,
            pending: None,
        });
    }

    pub(super) fn finish_layout_item(&mut self) {
        if let Some(flow) = &mut self.flow {
            if let Some(rect) = flow.pending.take() {
                flow.items.push(Item {
                    measure: Measure {
                        size: rect.size(),
                        flexible: false,
                        gap: false,
                    },
                    origin: rect.min,
                    placement: Some(self.context.end_placement()),
                });
            }
        }
    }

    pub(crate) fn finish_layout(&mut self) {
        self.finish_layout_item();
        let Some(flow) = self.flow.take() else { return };
        let measured: Vec<_> = flow.items.iter().map(|i| i.measure).collect();
        let direction = self.layout.direction;
        let sizes = resolve(
            &measured,
            direction,
            direction.main(self.layout.bounds.size()),
            self.layout.spacing,
        );
        let cross = measured
            .iter()
            .map(|m| direction.cross(m.size))
            .fold(0.0_f32, f32::max);
        let aligned_cross = if direction == Layout::Vertical {
            self.layout.bounds.size().x.max(cross)
        } else {
            cross
        };
        let needs_repaint = flow.items.iter().zip(&sizes).any(|(item, size)| {
            item.measure.flexible
                && item.placement.is_some()
                && (direction.main(item.measure.size) - *size).abs() > 0.01
        });
        let mut main = 0.0;
        let mut first = true;
        for (item, size) in flow.items.into_iter().zip(&sizes) {
            if !item.measure.gap {
                if !first {
                    main += self.layout.spacing;
                }
                first = false;
            }
            let target = self.layout.bounds.min
                + direction.size(
                    main,
                    flow.align
                        .offset(aligned_cross - direction.cross(item.measure.size)),
                );
            if let Some(placement) = item.placement {
                let clip = if item.measure.flexible {
                    self.clip.intersect(Rect::from_min_size(
                        self.layout.bounds.min + direction.size(main, 0.0),
                        direction.size(*size, aligned_cross),
                    ))
                } else {
                    self.clip
                };
                self.context.place(placement, target - item.origin, clip);
            }
            main += size;
        }
        let used_cross = if flow.align != Align::Start && direction == Layout::Vertical {
            aligned_cross
        } else {
            cross
        };
        self.layout.used = direction.size(main, used_cross);
        self.layout.cursor = self.layout.bounds.min
            + direction.size(main + if first { 0.0 } else { self.layout.spacing }, 0.0);
        // Fill content needs its resolved extent when built. Repaint only if that extent changed.
        if needs_repaint {
            self.context.request_repaint();
        }
        self.context.layouts.insert(
            self.scope,
            FlowState {
                last_frame: self.context.frame,
                measured,
            },
        );
    }

    /// Capture a component as one layout item, including components with multiple allocations.
    pub(crate) fn layout_item<R>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.layout_item_inner(false, build)
    }

    pub(super) fn layout_item_inner<R>(
        &mut self,
        flexible: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.finish_layout_item();
        if self.flow.is_none() && !flexible {
            return build(self);
        }
        let origin = self.layout.cursor;
        let direction = self.layout.direction;
        let mut available = (self.layout.bounds.max - origin).max(Vec2::ZERO);
        // Measure fixed siblings independently of provisional fill sizes.
        if let Some(flow) = &self.flow {
            let provisional: f32 = flow
                .items
                .iter()
                .filter(|i| i.measure.flexible)
                .map(|i| direction.main(i.measure.size))
                .sum();
            available = direction
                .size(
                    direction.main(self.layout.bounds.max - origin) + provisional,
                    direction.cross(available),
                )
                .max(Vec2::ZERO);
        }
        if flexible {
            let flow = self.flow.as_ref().expect("fill requires a layout scope");
            let sizes = resolve(
                &flow.previous,
                direction,
                direction.main(self.layout.bounds.size()),
                self.layout.spacing,
            );
            let main = sizes
                .get(flow.items.len())
                .copied()
                .unwrap_or(direction.main(available));
            available = direction.size(main, direction.cross(available));
        }
        let parent_clip = self.clip_rect();
        self.context.begin_placement(self.window);
        self.context.visual_depth += 1;
        self.context.visual_clips.push((self.window, parent_clip));
        let mut child = Ui {
            context: self.context,
            window: self.window,
            scope: self.scope,
            sequence: self.sequence,
            clip: Rect::from_min_size(Vec2::splat(-1.0e9), Vec2::splat(2.0e9)),
            layout: LayoutCursor::new(
                Rect::from_min_size(origin, available),
                direction,
                self.layout.spacing,
            ),
            enabled: self.enabled,
            backdrop_blur: self.backdrop_blur,
            hover_style: self.hover_style,
            flow: None,
        };
        child.layout.preferred_width = self.layout.preferred_width;
        let result = build(&mut child);
        child.finish_layout();
        let size = child.layout.used;
        self.sequence = child.sequence;
        self.context.visual_depth -= 1;
        self.context.visual_clips.pop();
        let placement = self.context.end_placement();
        self.layout.allocate(size);
        self.flow.as_mut().unwrap().items.push(Item {
            measure: Measure {
                size,
                flexible,
                gap: false,
            },
            origin,
            placement: Some(placement),
        });
        result
    }

    /// Divide remaining main-axis space equally between spacers and fill scopes.
    pub fn spacer(&mut self) {
        self.finish_layout_item();
        let origin = self.layout.cursor;
        self.layout.allocate(Vec2::ZERO);
        self.flow
            .as_mut()
            .expect("spacer requires a layout scope")
            .items
            .push(Item {
                measure: Measure {
                    size: Vec2::ZERO,
                    flexible: true,
                    gap: false,
                },
                origin,
                placement: None,
            });
    }
}
