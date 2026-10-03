//! Bound recursive XML expansion and resvg isolated-layer scratch before rendering.
use super::{ImageError, ImageLimits};
use resvg::usvg::{self, roxmltree};
use std::collections::HashMap;

pub(super) fn xml(doc: &roxmltree::Document, limits: &ImageLimits) -> Result<(), ImageError> {
    let ids: HashMap<_, _> = doc
        .descendants()
        .filter_map(|n| n.attribute("id").map(|id| (id, n)))
        .collect();
    let mut memo = HashMap::new();
    let mut stack = Vec::new();
    fn weight<'a>(
        node: roxmltree::Node<'a, 'a>,
        ids: &HashMap<&'a str, roxmltree::Node<'a, 'a>>,
        memo: &mut HashMap<roxmltree::NodeId, usize>,
        stack: &mut Vec<roxmltree::NodeId>,
        budget: usize,
    ) -> Result<usize, ImageError> {
        if let Some(&w) = memo.get(&node.id()) {
            return Ok(w);
        }
        if stack.len() >= 64 || stack.contains(&node.id()) {
            return Err(ImageError(
                "SVG recursive references or nesting exceed limits".into(),
            ));
        }
        stack.push(node.id());
        let mut sum = 512usize;
        for a in node.attributes() {
            sum = sum.saturating_add(a.value().len().saturating_mul(32));
        }
        for child in node.children() {
            sum = sum.saturating_add(weight(child, ids, memo, stack, budget)?);
            if sum > budget {
                return Err(ImageError(
                    "SVG expanded geometry exceeds memory budget".into(),
                ));
            }
        }
        if node.has_tag_name("use") {
            let href = node
                .attribute("href")
                .or_else(|| node.attribute(("http://www.w3.org/1999/xlink", "href")));
            if let Some(target) = href
                .and_then(|h| h.strip_prefix('#'))
                .and_then(|id| ids.get(id))
            {
                sum = sum.saturating_add(weight(*target, ids, memo, stack, budget)?);
            }
        }
        stack.pop();
        if sum > budget {
            return Err(ImageError(
                "SVG expanded geometry exceeds memory budget".into(),
            ));
        }
        memo.insert(node.id(), sum);
        Ok(sum)
    }
    weight(
        doc.root_element(),
        &ids,
        &mut memo,
        &mut stack,
        limits.cpu_cache_bytes.min(limits.max_decoded_bytes),
    )?;
    Ok(())
}
pub(super) fn raster(
    tree: &usvg::Tree,
    size: [u32; 2],
    limits: &ImageLimits,
) -> Result<(), ImageError> {
    let sx = size[0] as f64 / tree.size().width() as f64;
    let sy = size[1] as f64 / tree.size().height() as f64;
    fn scratch(group: &usvg::Group, sx: f64, sy: f64, depth: usize) -> Result<u64, ImageError> {
        if depth > 64 {
            return Err(ImageError("SVG layer nesting exceeds limit".into()));
        }
        let bounds = group.abs_layer_bounding_box();
        let own = if group.should_isolate() {
            // Conservative allowance for pixmaps, clipping/masks, and AA padding.
            ((bounds.width() as f64 * sx).ceil() + 6.0)
                * ((bounds.height() as f64 * sy).ceil() + 6.0)
                * 16.0
        } else {
            0.0
        };
        if !own.is_finite() || own > u64::MAX as f64 {
            return Err(ImageError("SVG layer dimensions overflow".into()));
        }
        let mut nested = 0;
        for node in group.children() {
            let g = match node {
                usvg::Node::Group(g) => Some(g.as_ref()),
                usvg::Node::Text(t) => Some(t.flattened()),
                _ => None,
            };
            if let Some(g) = g {
                nested = nested.max(scratch(g, sx, sy, depth + 1)?);
            }
        }
        Ok((own as u64).saturating_add(nested))
    }
    let bytes = limits.check_size(size)? as u64;
    if bytes.saturating_add(scratch(tree.root(), sx, sy, 0)?) > limits.max_decoded_bytes as u64 {
        return Err(ImageError(
            "SVG isolated layers exceed worker memory limit; reduce raster size".into(),
        ));
    }
    Ok(())
}
