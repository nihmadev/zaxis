# Contributing

Report bugs and suggest features through [GitHub Issues](https://github.com/nihmadev/zaxis/issues).
You can also contact [@nihmadev on Telegram](https://t.me/nihmadev).
Discuss changes that alter public APIs or architecture before opening a large PR.

## Development

Use Rust 1.90 or newer. Run from the repository root:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo check --workspace --all-targets --locked
cargo test --workspace --lib --tests --examples --locked
cargo test --workspace --doc --locked
```

Clippy CI uses Rust 1.98.0 with the root `clippy.toml`; the minimum supported compiler
remains 1.90. Dependency policy is in `deny.toml`: run `cargo deny --locked check`.
Duplicate dependency versions are warnings; vulnerabilities, yanked releases,
unapproved licenses, and unknown sources fail the check. The unmaintained
`ttf-parser` advisory has a documented temporary exception pending upstream migration.

Root `lychee.toml` configures documentation link checks. With lychee installed:

```sh
lychee --config lychee.toml --offline '*.md' 'docs/content/**/*.mdx' 'crates/*/README.md'
```

Local links are checked on documentation changes. External links are checked weekly
and through the Documentation links workflow's manual run, separately from release CI.

`cargo test --all-targets` runs the custom benchmark harness; use the commands above
for correctness checks and `cargo bench --bench performance` for benchmarks.
GPU tests marked ignored and native smoke tests require a working graphics adapter
and, for windows, a desktop session. CI sets `ZAXIS_SKIP_GPU_TESTS=1` and does not
validate native rendering. For visual changes, run the relevant example and attach
a screenshot, the OS, GPU/backend, and DPI scale to the PR.

Optional-feature changes should also pass `cargo check --lib --no-default-features --locked`.
To check the documented minimum version, prefix Cargo commands with `cargo +1.90.0`.

Keep fixes in the library rather than compensating in examples. Put components in
`src/components/`, preserve public compatibility exports, and use focused modules.
New or substantially changed Rust source files should stay within 430 lines; split
larger modules by responsibility. Add regression tests for behavior changes and
update the relevant documentation and examples.

`crates/z-emoji` contains the complete bundled emoji font and its OFL license.
Keep font data out of the main crate so both uploads stay below crates.io's size limit.

For documentation changes, use Node.js 22 or newer (CI uses 24):

```sh
cd docs
npm ci
npm run typecheck
npm run build
```

## Pull requests

Explain the problem, the resulting behavior, and the checks you ran. Keep unrelated
changes out of the PR. Follow the existing style and include required asset licenses.
Development releases may change public APIs; mention breaking changes explicitly.
Contributions are licensed under the project's MIT license.
