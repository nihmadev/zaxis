//! `Images`: a central slide with its neighbours behind it, scaled, lowered and dimmed.
use super::style::Look;
use super::{
    photo::photo,
    pose::{depth_order, slide_pose, SlideGeometry},
    region::{safe_content, with_region},
    scene::{bounds, Content, Painted, Scene},
    surface::paint_card,
};
use crate::{Blur, Border, Color, CornerRadius, Rect, Shape, Ui, Vec2};

fn along(axis: usize, amount: f32) -> Vec2 {
    if axis == 0 {
        Vec2::new(amount, 0.0)
    } else {
        Vec2::new(0.0, amount)
    }
}

pub(super) fn geometry(area: Rect, look: &Look, count: usize, axis: usize) -> SlideGeometry {
    // Neighbours need room beside the slide; fewer pages need less of it.
    let reach = look.neighbors().min(count.saturating_sub(1)) as f32;
    let inset = (look.peek() * reach).min(area.size()[axis] * 0.3);
    SlideGeometry {
        slide: Rect::from_min_max(area.min + along(axis, inset), area.max - along(axis, inset)),
        axis,
        neighbors: look.neighbors(),
        peek: if reach > 0.0 { inset / reach } else { 0.0 },
        scale_step: look.slide_scale_step(),
        drop: look.slide_drop(),
        dim: look.dim(),
        blur: look.blur(),
    }
}

fn shade(rect: Rect, rounding: f32, alpha: f32) -> Shape {
    Shape::Rect {
        rect,
        fill: Color::rgba(0, 0, 0, (alpha.clamp(0.0, 1.0) * 255.0).round() as u8),
        rounding: CornerRadius::all(rounding),
        border: Border::NONE,
    }
}

pub(super) fn paint(ui: &mut Ui<'_>, scene: &Scene<'_>, content: &mut Content<'_>) -> Painted {
    let g = geometry(scene.area, scene.look, scene.model.count, scene.axis);
    let look = scene.look;
    let rounding = look.rounding();
    let extent = g.slide.size()[scene.axis] * look.travel();
    let mut painted = Painted {
        layers: Vec::new(),
        front: g.slide,
    };
    for (k, page, d) in scene.layers(g.neighbors, g.neighbors, depth_order) {
        let Some(pose) = slide_pose(&g, d) else {
            continue;
        };
        let transform = scene.overscroll(k, pose.transform, extent);
        let key = scene.model.key(page);
        let scope = scene.id.with(("page", key));
        let front = scene.interactive(k);
        let slide = g.slide;
        let near = (1.0 - d.abs()).clamp(0.0, 1.0);
        with_region(ui, scope, slide, scene.stage, scene.enabled, |layer| {
            layer.visual_gated("layer", transform, pose.opacity, Some(front), |inner| {
                let current = k == scene.target;
                let node = super::access::page(inner, scope, page, scene.model.count, current);
                match content {
                    Content::Pages(build) => {
                        paint_card(inner, scope.with("card"), slide, look, 0.0);
                        let safe = safe_content(slide, look.content_padding(), rounding);
                        with_region(inner, scope.with("content"), safe, safe, true, |page_ui| {
                            build(page_ui, page)
                        });
                    }
                    Content::Photos { source, overlay } => {
                        photo(inner, scope.with("photo"), slide, source(page), rounding);
                        if let Some(overlay) = overlay.as_mut().filter(|_| near > 0.01) {
                            let scrim = look.scrim() * near;
                            if scrim > 0.003 {
                                let id = scope.with("scrim");
                                inner.context.paint(
                                    id,
                                    inner.window,
                                    inner.clip,
                                    vec![crate::context::Paint::Shape(shade(
                                        slide, rounding, scrim,
                                    ))],
                                );
                            }
                            let safe = safe_content(slide, look.content_padding(), rounding);
                            with_region(inner, scope.with("overlay"), safe, safe, true, |u| {
                                overlay(u, page)
                            });
                        }
                    }
                }
                if pose.dim > 0.003 {
                    inner.context.paint(
                        scope.with("dim"),
                        inner.window,
                        inner.clip,
                        vec![crate::context::Paint::Shape(shade(
                            slide, rounding, pose.dim,
                        ))],
                    );
                }
                if pose.blur >= 0.5 {
                    Blur::new(slide)
                        .radius(pose.blur)
                        .corner_radius(CornerRadius::all(rounding))
                        .show(inner);
                }
                inner.a11y_end(node, Some(slide));
                inner.allocate_space(slide.size());
            });
        });
        let rect = bounds(transform, slide);
        if k == scene.target {
            painted.front = rect;
        }
        painted.layers.push((k, rect));
    }
    painted
}
