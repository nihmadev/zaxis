use super::*;
use crate::Align;

#[derive(Clone, Copy)]
enum SizeRule {
    Content,
    Exact(f32),
    Min(f32),
    Max(f32),
}
impl SizeRule {
    fn available(self, available: f32) -> f32 {
        match self {
            Self::Exact(n) => n,
            Self::Min(n) => available.max(n),
            Self::Max(n) => available.min(n),
            Self::Content => available,
        }
    }
    fn occupied(self, used: f32) -> f32 {
        match self {
            Self::Exact(n) => n,
            Self::Min(n) => used.max(n),
            Self::Max(n) => used.min(n),
            Self::Content => used,
        }
    }
}

impl Ui<'_> {
    /// Align row items vertically within the tallest item. Rows do not wrap.
    pub fn horizontal_aligned<R>(
        &mut self,
        align: Align,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.layout_scope(
            Layout::Horizontal,
            "row",
            align,
            SizeRule::Content,
            SizeRule::Content,
            false,
            build,
        )
    }

    /// Align column items horizontally within the available column width.
    pub fn vertical_aligned<R>(&mut self, align: Align, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.layout_scope(
            Layout::Vertical,
            "column",
            align,
            SizeRule::Content,
            SizeRule::Content,
            false,
            build,
        )
    }

    /// Reserve a share of remaining main-axis space for a column of content.
    /// Content callbacks execute once per pass; changed intrinsic sizes can schedule a redraw.
    pub fn fill<R>(&mut self, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.layout_scope(
            Layout::Vertical,
            "fill",
            Align::Start,
            SizeRule::Content,
            SizeRule::Content,
            true,
            build,
        )
    }

    fn layout_scope<R>(
        &mut self,
        direction: Layout,
        kind: &'static str,
        align: Align,
        width: SizeRule,
        height: SizeRule,
        flexible: bool,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        let id = self.next_id(kind);
        let parent_direction = self.layout.direction;
        self.layout_item_inner(flexible, |ui| {
            // layout_item uses the same Ui when the parent has no flow.
            let bounds = Rect::from_min_max(ui.layout.cursor, ui.layout.bounds.max);
            let size = Vec2::new(
                width.available(bounds.size().x.max(0.0)),
                height.available(bounds.size().y.max(0.0)),
            );
            let mut child = Ui {
                context: ui.context,
                window: ui.window,
                scope: id,
                sequence: 0,
                clip: ui.clip,
                layout: LayoutCursor::new(
                    Rect::from_min_size(bounds.min, size),
                    direction,
                    ui.layout.spacing,
                ),
                enabled: ui.enabled,
                backdrop_blur: ui.backdrop_blur,
                hover_style: ui.hover_style,
                flow: None,
            };
            if matches!(width, SizeRule::Exact(_) | SizeRule::Max(_))
                || matches!(height, SizeRule::Exact(_) | SizeRule::Max(_))
            {
                child.clip = child.clip.intersect(Rect::from_min_size(bounds.min, size));
            }
            child.layout.preferred_width = if matches!(width, SizeRule::Exact(_))
                || (flexible && parent_direction == Layout::Horizontal)
            {
                Some(size.x)
            } else {
                ui.layout.preferred_width
            };
            child.begin_layout(align);
            let result = build(&mut child);
            child.finish_layout();
            let mut used = Vec2::new(
                width.occupied(child.layout.used.x),
                height.occupied(child.layout.used.y),
            );
            if flexible {
                used = parent_direction
                    .size(parent_direction.main(size), parent_direction.cross(used));
            }
            ui.allocate_space(used);
            result
        })
    }

    /// Reserve an exact width; overflowing content is clipped by the enclosing UI.
    pub fn with_width<R>(&mut self, width: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.sized_scope(
            SizeRule::Exact(super::super::columns::dimension(width)),
            SizeRule::Content,
            build,
        )
    }
    pub fn with_min_width<R>(&mut self, width: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.sized_scope(
            SizeRule::Min(super::super::columns::dimension(width)),
            SizeRule::Content,
            build,
        )
    }
    pub fn with_max_width<R>(&mut self, width: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.sized_scope(
            SizeRule::Max(super::super::columns::dimension(width)),
            SizeRule::Content,
            build,
        )
    }
    pub fn with_height<R>(&mut self, height: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.sized_scope(
            SizeRule::Content,
            SizeRule::Exact(super::super::columns::dimension(height)),
            build,
        )
    }
    pub fn with_min_height<R>(&mut self, height: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.sized_scope(
            SizeRule::Content,
            SizeRule::Min(super::super::columns::dimension(height)),
            build,
        )
    }
    pub fn with_max_height<R>(&mut self, height: f32, build: impl FnOnce(&mut Ui<'_>) -> R) -> R {
        self.sized_scope(
            SizeRule::Content,
            SizeRule::Max(super::super::columns::dimension(height)),
            build,
        )
    }
    fn sized_scope<R>(
        &mut self,
        width: SizeRule,
        height: SizeRule,
        build: impl FnOnce(&mut Ui<'_>) -> R,
    ) -> R {
        self.layout_scope(
            self.layout.direction,
            "size",
            Align::Start,
            width,
            height,
            false,
            build,
        )
    }
}
