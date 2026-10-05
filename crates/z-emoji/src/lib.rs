//! Unmodified Noto Color Emoji font bytes, bundled for offline desktop rendering.
//!
//! zaxis enables this crate through its default `bundled-emoji` feature.
//! This crate only exposes static font data; it performs no loading or rasterization.
//! The font and this data crate are distributed under the SIL Open Font License 1.1.

#![no_std]
#![forbid(unsafe_code)]

/// Complete bundled Noto Color Emoji TrueType font, including bitmap glyphs.
pub static FONT_DATA: &[u8] = include_bytes!("../assets/NotoColorEmoji.ttf");
