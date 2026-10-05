//! Text entering the field: line-break normalization, length limits, and the cheap
//! content fingerprint that detects edits made by the application.
use unicode_segmentation::UnicodeSegmentation;

const K: u64 = 0x9E37_79B9_7F4A_7C15;

/// Fast non-cryptographic hash over 32-byte blocks; four independent lanes keep the
/// multiplier chains overlapped, so large documents hash at memory speed.
pub(crate) fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut lanes = [K, K.rotate_left(16), K.rotate_left(32), K.rotate_left(48)];
    let (blocks, rest) = bytes.as_chunks::<32>();
    for block in blocks {
        for (lane, chunk) in lanes.iter_mut().zip(block.as_chunks::<8>().0) {
            *lane = (lane.rotate_left(23) ^ u64::from_le_bytes(*chunk)).wrapping_mul(K);
        }
    }
    let mut hash = bytes.len() as u64;
    for lane in lanes {
        hash = (hash.rotate_left(29) ^ lane).wrapping_mul(K);
    }
    let (words, tail) = rest.as_chunks::<8>();
    for chunk in words {
        hash = (hash.rotate_left(23) ^ u64::from_le_bytes(*chunk)).wrapping_mul(K);
    }
    let tail = tail
        .iter()
        .enumerate()
        .fold(0_u64, |t, (i, b)| t | u64::from(*b) << (8 * i));
    (hash.rotate_left(23) ^ tail).wrapping_mul(K) ^ hash >> 32
}

/// Identity of a string's content for external-change detection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Fingerprint(usize, u64);

impl Fingerprint {
    pub fn of(text: &str) -> Self {
        Self(text.len(), hash_bytes(text.as_bytes()))
    }
}

/// Single-line input: no control characters at all.
pub(crate) fn single_line(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

/// Multi-line input: `\r\n` and `\r` become `\n`, tabs stay, other controls are dropped.
pub(crate) fn multi_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push('\n');
            }
            '\n' | '\t' => out.push(c),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// Truncate `value` so that replacing `selected` bytes of `current` stays within `max`
/// characters, cutting between grapheme clusters.
pub(crate) fn fit(current: &str, selected: &str, value: String, max: Option<usize>) -> String {
    let Some(max) = max else { return value };
    // Characters never outnumber bytes, so a short document needs no exact count.
    if current.len() + value.len() - selected.len().min(current.len()) <= max {
        return value;
    }
    let used = current.chars().count() - selected.chars().count();
    let room = max.saturating_sub(used);
    if value.chars().count() <= room {
        return value;
    }
    let mut kept = 0;
    let mut end = 0;
    for (i, g) in value.grapheme_indices(true) {
        kept += g.chars().count();
        if kept > room {
            break;
        }
        end = i + g.len();
    }
    value[..end].to_owned()
}
