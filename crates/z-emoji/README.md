# z-emoji

Unmodified Noto Color Emoji font data for [zaxis](https://github.com/nihmadev/zaxis).
This `no_std` crate exposes `FONT_DATA: &[u8]`; it has no runtime dependencies and
performs no font discovery, decoding, rasterization, or network access.

zaxis uses it through the default `bundled-emoji` feature. Disable that feature
when you prefer system fonts and do not want the bundled emoji download or binary size.

```rust
let font_bytes: &[u8] = z_emoji::FONT_DATA;
```

The font is distributed under the [SIL Open Font License 1.1](assets/OFL-NotoEmoji.txt).
Include that license when redistributing the font. Bugs and contributions are handled
in the main zaxis repository; contact [@nihmadev on Telegram](https://t.me/nihmadev)
for sensitive vulnerability reports.
