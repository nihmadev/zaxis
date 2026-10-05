use zaxis::components::segmented::layout::{plan, snap, Item, Metrics, Request};
use zaxis::components::segmented::options::SegmentWidth;
use zaxis::{Context, FontWeight};

fn metrics(scale: f32) -> Metrics {
    Metrics {
        font: 14.0,
        weight: FontWeight::default(),
        pad_x: 12.0,
        icon: 16.0,
        icon_gap: 6.0,
        seg_h: snap(26.0, scale),
        padding: snap(3.0, scale),
        gap: snap(2.0, scale),
        scale,
    }
}

fn items(texts: &[&str], icon: bool) -> Vec<Item> {
    texts
        .iter()
        .map(|t| Item {
            text: (*t).to_owned(),
            has_icon: icon,
        })
        .collect()
}

fn request(width: SegmentWidth, available: f32) -> Request {
    Request {
        width,
        min_width: 0.0,
        vertical: false,
        icon_only: false,
        reserve_mark: false,
        available,
    }
}

#[test]
fn shrunk_rows_fill_the_container_exactly_at_every_scale() {
    let mut c = Context::new();
    for scale in [1.0, 1.25, 1.5, 2.0, 1.75] {
        for available in [137.3, 201.9, 250.0, 313.77] {
            let m = metrics(scale);
            let it = items(&["Overview", "Activity", "Settings"], false);
            let p = plan(&mut c, &it, &m, &request(SegmentWidth::Equal, available));
            let px = |v: f32| (v * scale).round() as i64;
            let row: i64 =
                p.widths.iter().map(|w| px(*w)).sum::<i64>() + px(m.gap) * 2 + 2 * px(m.padding);
            assert!(
                row <= (available * scale).floor() as i64
                    && (p.size.x * scale).round() as i64 == row,
                "scale {scale}, width {available}: row {row}px, size {}",
                p.size.x
            );
        }
    }
}

#[test]
fn fill_uses_all_the_space_with_no_drift() {
    let mut c = Context::new();
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let m = metrics(scale);
        let it = items(&["A", "B", "C", "D", "E", "F", "G"], false);
        let p = plan(&mut c, &it, &m, &request(SegmentWidth::Fill, 333.3));
        let total = (p.size.x * scale).round() as i64;
        assert_eq!(total, (333.3_f32 * scale).floor() as i64, "scale {scale}");
    }
}

#[test]
fn squeezed_labels_are_cut_with_an_ellipsis_then_hidden_for_icon_segments() {
    let mut c = Context::new();
    let m = metrics(1.0);
    let text_only = items(&["Internationalization", "Localization"], false);
    let p = plan(&mut c, &text_only, &m, &request(SegmentWidth::Equal, 160.0));
    let label = p.labels[0].as_ref().expect("a cut label stays visible");
    assert!(label.truncated && label.text.ends_with('…'));
    assert!(label.size.x <= p.widths[0] - 2.0 * m.pad_x + 0.01);
    let with_icons = items(&["Internationalization", "Localization"], true);
    let p = plan(
        &mut c,
        &with_icons,
        &m,
        &request(SegmentWidth::Equal, 120.0),
    );
    assert!(
        p.labels.iter().all(Option::is_none),
        "icons remain, labels go"
    );
    assert!(p.size.x <= 120.0 + 0.01);
    let roomy = plan(
        &mut c,
        &with_icons,
        &m,
        &request(SegmentWidth::Equal, 800.0),
    );
    assert!(roomy
        .labels
        .iter()
        .all(|l| l.as_ref().is_some_and(|l| !l.truncated)));
}

#[test]
fn width_modes_size_segments_as_documented() {
    let mut c = Context::new();
    let m = metrics(1.0);
    let it = items(&["I", "Wider label"], false);
    let equal = plan(&mut c, &it, &m, &request(SegmentWidth::Equal, 800.0));
    assert_eq!(equal.widths[0], equal.widths[1]);
    let content = plan(&mut c, &it, &m, &request(SegmentWidth::Content, 800.0));
    assert!(content.widths[0] < content.widths[1]);
    let fixed = plan(&mut c, &it, &m, &request(SegmentWidth::Fixed(90.0), 800.0));
    assert!(fixed.widths.iter().all(|w| (*w - 90.0).abs() < 0.01));
}
