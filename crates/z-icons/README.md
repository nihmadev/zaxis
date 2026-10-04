# z-icons

[Lucide](https://lucide.dev) icons as static SVG data for [zaxis](https://github.com/nihmadev/zaxis).

```rust
// With zaxis' `bundled-icons` feature:
ui.add(zaxis::Image::new(&zaxis::icons::USER).size(zaxis::vec2(16.0, 16.0)).tint(color));
```

Icons are `static` items, so the linker drops those you do not use. Strokes are white; set the
color with the image tint.

Regenerate from a Lucide checkout: `python crates/z-icons/generate.py <path-to-lucide>`.

Lucide is ISC licensed; some icons derive from Feather (MIT). See `LICENSE-LUCIDE`.
