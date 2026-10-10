# z-icons

1,866 [Lucide](https://lucide.dev) icons for Rust applications, independent of any GUI
framework. Use them in desktop applications, web pages, embedded displays, or asset exporters.
The library has **zero dependencies**, supports `no_std`, and requires Rust 1.90+.
It provides SVG data; your application chooses the renderer and owns its resources.

## Install

```toml
[dependencies]
z-icons = "0.0.2"
```
Or use:

```bash
cargo add z-icons
```

## Embed an icon

```rust
use z_icons::USER;

let bytes: &'static [u8] = USER.svg();
let text: &'static str = USER.svg_str();
assert_eq!(USER.name(), "user");
```

Pass `bytes` or `text` to your application's SVG loader. Both accessors borrow static data
without allocation. Original bundled SVGs have **white strokes**, suitable for renderers
that multiply image pixels by a tint. Renderers that do not tint should use an explicit color:

```rust
let svg = z_icons::SETTINGS.render()
    .color("#2563eb")
    .size(32, 32)
    .stroke_width(1.5)
    .to_string();
// Pass svg.as_bytes() to an SVG loader, or save the document as an asset.
```

`render()` uses `currentColor` by default, for inline SVG that inherits CSS color. An SVG
loaded as an external image does not inherit the surrounding page's color; set `.color(...)`
for that case. CSS color values are passed through and XML attribute characters are escaped.

Width and height initially preserve the original 24 × 24 size. `.size(width, height)` sets
positive pixel dimensions while retaining the viewBox and normal SVG aspect-ratio behavior.
Color applies to both strokes and the filled dots present in some Lucide icons.
Stroke width uses viewBox units; it scales with the image. Zero hides the stroke; negative
or non-finite widths and zero dimensions panic. The API does not parse or rasterize SVG.

Keep the SVG string and your toolkit's loaded image/texture in application state. Reuse them
between frames; changing color, size or stroke width creates a different document.

## Without an allocator

`Svg` implements `core::fmt::Display` and can write into a caller-provided `core::fmt::Write`
buffer. The library itself does not require `alloc` or `std`:

```rust
fn write_icon(output: &mut impl core::fmt::Write) -> core::fmt::Result {
    z_icons::CHECK.render().color("#fff").write_to(output)
}
```

Your renderer may have its own allocator or platform requirements.

## Names and optional discovery

Constants use the Lucide filename in `SCREAMING_SNAKE_CASE`: `user` → `USER`,
`arrow-left` → `ARROW_LEFT`, `grid-2x2` → `GRID_2X2`. Names beginning with a digit
get a leading underscore. See the generated Rust API for the names in this bundled snapshot.

By default there is no table referencing all icons, so the linker can discard unused data.
For icon pickers or names read from configuration, enable the optional catalog:

```toml
z-icons = { path = "path/to/zaxis/crates/z-icons", features = ["catalog"] }
```

```rust
# #[cfg(feature = "catalog")] {
let icon = z_icons::get("arrow-left").expect("known name");
assert_eq!(icon.name(), "arrow-left");
for icon in z_icons::all() {
    println!("{}", icon.name());
}
# }
```

`get()` uses binary search over exact, case-sensitive, kebab-case names; unknown names return
`None`. `all()` is sorted by name. Using either function retains the complete icon set in
that binary. Default features are empty, and the catalog also works with `no_std`.

## Your own static icons

```rust
static BRAND: z_icons::Icon = z_icons::Icon::new("brand", include_str!("brand.svg"));
```

Raw access accepts any static UTF-8 SVG document. For `render()`, the source must start with
an `<svg ...>` root, allowing leading whitespace, without an XML declaration or doctype.
Root customization preserves the original viewBox, geometry and other attributes. Child
strokes and CSS styles follow SVG precedence and can override the root options. Custom data
is trusted application data and is not sanitized. A malformed root returns a formatting
error from `write_to()`; `to_string()` can panic on that error.

## zaxis integration

zaxis is one optional consumer, with its adapter implemented in the zaxis crate. Enable
its `bundled-icons` feature and keep using the existing API:

```rust,ignore
use zaxis::{icons, vec2, Image};
ui.add(Image::new(&icons::USER).size(vec2(16.0, 16.0)).tint(color));
```

No zaxis feature or dependency is required by users of `z-icons` itself.

## Example and development

Run these commands from the repository root:

```sh
cargo run -p z-icons --example export -- '#2563eb' 48 > target/user.svg
cargo test -p z-icons --all-features --locked
cargo check -p z-icons --lib --no-default-features --locked
cargo clippy -p z-icons --all-targets --all-features --locked -- -D warnings
cargo package -p z-icons --allow-dirty --locked
```

The example exports a real SVG using only `z-icons` and the standard library. Tests use
resvg as a **development-only** dependency to validate geometry, color, sizing and every
bundled asset; it is not linked into applications that depend on this crate.

`python crates/z-icons/generate.py <path-to-lucide-checkout>` regenerates `icons/` and
`src/generated.rs`. Generation is a maintainer operation: Python and a Lucide checkout are
not needed to build or use the crate. Generated sources and SVG files ship in the package.
The package also includes its own tests, example, generator, changelog and licenses.

## License

Rust code and the generator are [MIT licensed](LICENSE). Lucide assets are ISC licensed;
some derive from Feather (MIT). See [LICENSE-LUCIDE](LICENSE-LUCIDE) and include the
applicable notices when redistributing the assets.
