# Working with zaxis

These instructions apply to the entire repository. Nested AGENTS.md files refine the rules for their directories; explicit user instructions take precedence.

## Working approach

- Work autonomously: take a clear task through to a working result, choosing a simple, practical solution yourself.
- Inspect the existing implementation first. Search with `rg` and trace public API → state → layout/input → rendering → consumer. Do not invent library capabilities.
- Check `git status` before editing and preserve other people's uncommitted changes. Do not revert, reformat, or clean unrelated files.
- Fix the cause in the shared component, Context, or renderer. Do not hide library defects with example-specific workarounds.
- Avoid unnecessary scripts, backups, dependencies, and abstractions. Plan complex tasks, not every edit.
- Ask only when ambiguity materially affects the result or before actions with major, hard-to-reverse consequences: destroying data/history, sending private data externally, publishing secrets, or changing production systems.
- Communicate briefly. Finish with the result, checks performed, and specific remaining limitations.

## Project structure

zaxis is a Rust desktop immediate mode GUI with retained interaction state and cached CPU/GPU geometry. Rust 1.90+ is required; see Cargo.toml for current dependencies and features.

- `src/lib.rs`: public exports and API compatibility.
- `src/components/`: components, Ui, styles, and themes.
- `src/context/`: input, focus, state, frame lifecycle, interactions, and caches.
- `src/layout.rs`, `src/shapes/`, `src/text/`: layout, geometry, and text.
- `src/protocol/`, `src/renderer/`, `src/shaders/`: DrawData, GPU resources, and WGSL.
- `src/app/`: winit runner, repaint scheduling, and GPU recovery.
- `src/animation/`, `src/images/`: existing animation and image-loading engines.
- `examples/`: examples using the actual public API.
- `tests/`: regression and integration checks; `benches/`: the shared performance harness.
- `docs/content/docs/`: documentation; `crates/zaxis-emoji/`: the separate font package and its licenses.

## Modularity and file size

- The target limit is **450 lines per file**, including comments and blank lines. Apply it to source code, shaders, tests, examples, and benchmarks; generated files and lockfiles are exempt.
- **451–550 lines** are allowed only as a justified exception when splitting would reduce cohesion. Briefly explain the reason in the final report. **More than 550 lines is prohibited** for new or modified handwritten code files.
- When substantially changing an existing oversized file, extract the affected responsibility. Avoid unrelated bulk refactoring solely to meet the limit.
- Split by responsibility: API/options, layout, input/events, paint, resources/cache, and tests. Do not mechanically divide code into arbitrary chunks to satisfy a counter.
- Keep parent modules compact: declarations, exports, and necessary coordination. Preserve existing public paths and re-exports when moving code.
- Prefer small, composable APIs, shared primitives, and focused customization. Do not introduce a universal framework or duplicate engine for one component.
- This 450/550 policy supersedes the older 430-line limit in the documentation.

## Components and correctness

- A component is ready when it works through the public API and standard runner, not merely when it appears in a demo.
- Every interactive element must perform its stated action and change real state. Do not present empty handlers, fake values, decorative functional-looking buttons, or stubs as completed features.
- For new components, check applicable behavior: hover/active/disabled states, focus and keyboard input, dragging, clipping, nested scrolling, DPI, resizing, opening/closing, and state cleanup.
- Use stable Id values. One user input should produce one change/event; do not duplicate callbacks or mutations.
- Layout, paint, hit testing, clipping, scrolling, popup portals, cached geometry, and IME must consistently account for offsets and sizes.
- Use the existing Popup and its layers for overlays; account for clipping by Window and nested containers.
- Use the existing animation engine and repaint deadlines. Respect reduced motion; settled or hidden animations must not keep requesting redraws indefinitely.
- Keep text measurement, rendering, grapheme boundaries, and IME consistent. Do not replace shaping with per-character layout.
- Resources and caches must have bounded growth and correct lifecycles. Check revision changes, partial uploads, and full-update fallbacks on the actual renderer path.
- Update the relevant documentation and example. If behavior is missing, state the limitation explicitly instead of presenting the component as complete.

## Demos and examples

- A demo should show the component and its interactions. Keep only necessary labels, data, and useful controls.
- Do not add marketing copy, lengthy explanations, repeated headings, technical status text, debug counters, or filler. Put API explanations in documentation and diagnostics in a separate mode or logs.
- Do not add controls without real functionality. Sample data is acceptable when actions on it work.
- Use library components and their standard styling. Do not manually draw substitutes or patch component bugs with local workarounds.
- Preserve normal scrolling, moving/resizing, focus, and content accessibility when window size or DPI changes.
- For visual changes, run the relevant example and inspect the result. A successful build or smoke test is not visual verification.

## Validation

Run all Cargo commands from the repository root. Choose checks appropriate to the change; do not repeat the entire suite without a reason.

```sh
cargo check --workspace --all-targets --locked
cargo test --workspace --lib --tests --examples --locked
cargo test --workspace --doc --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
```

- For a local fix, start with the relevant test target/filter. Add regression coverage for changed behavior; avoid tests that merely repeat the implementation.
- For optional-feature changes, use `cargo check --lib --no-default-features --locked`; check MSRV with `cargo +1.90.0` when the change affects it.
- `cargo test --all-targets` includes the custom benchmark harness. Use separate test targets for correctness and `cargo bench` for performance.
- Native/GPU checks require a desktop session and a working adapter. `ZAXIS_SKIP_GPU_TESTS=1` and successful CI do not verify native rendering.
- Example invocation: `cargo run --release --example settings -- --smoke-test`. Use this flag only in examples that implement it; omit it for visual inspection.
- On Windows, if LNK1104 occurs, check whether the corresponding example executable is still open, close it, and retry the failed command.
- Distinguish baseline failures from regressions introduced by the change. Do not hide failures with unexplained skips or describe partial coverage as full verification.
- For documentation, follow CONTRIBUTING.md: run typecheck/build commands from `docs`. Rust builds are unnecessary for instruction-only changes.

## Benchmarking complex components

- Every new or substantially changed complex component must have scenarios in the existing `benches/performance.rs` harness and relevant `benches/support/` modules. Do not create a parallel measurement system.
- Measure the public API and real behavior. Cover idle/cache reuse, interactions, data changes, load scaling, and lifecycle; add scrolling/virtualization, popups, text, animation, or uploads as relevant to the component.
- Verify resulting state, geometry, and relevant cache/resource counters. A fast but broken component is not a successful result. Run assertions outside timed sections.
- Start with a focused quick run; use `--hard` for hot-path changes, scalability, and final validation of substantial optimizations.
- List cases and flags with `cargo bench --bench performance -- --list` and `--help`. Replace `<case>` below with an actual case name/filter.

```sh
cargo bench --bench performance -- --quick --cpu-only --filter <case>
cargo bench --bench performance -- --quick --gpu-only --filter <case>
cargo bench --bench performance -- --hard --filter <case> --output target/component-benchmark.json
```

- Set load with `--sizes N,N,...`; cover small, typical, and stress scenes. Diagnostic flags include `--gpu-wait` and `--gpu-timestamps`; enforce a specified budget with `--max-p95-ms N`.
- Compare before/after using identical cases, release profile, warmup, sample count, load, resolution, DPI, backend, and presentation mode. One noisy quick run does not demonstrate an improvement.
- Report p50/p95/p99 and relevant counters: rebuilds, geometry/texture uploads, mesh sizes, and resources/memory. Record OS, GPU/backend, Vsync/Immediate mode, and any fallback.
- Separate CPU layout/input/paint, encoding/submission/present, and actual GPU execution time. CPU time spent calling the renderer is not GPU execution time.
- `--gpu-wait` serializes frames; timestamps/readback use a diagnostic path. Do not present these measurements as the normal asynchronous production path or the cost of `queue.write_texture` without measuring that operation.
- Synthetic input measures dispatcher → model/frame latency, not full OS-to-display latency. Label memory estimates as estimates and unavailable GPU metrics as unavailable.
- Check `completed`, `failure`, `verified`, and `unsupported` in the JSON, not merely whether the file exists. Save reports under `target/`; add a substantive report to docs only when required by the task.

## Completion criteria

The implementation works in the library, its public API and example agree, and affected behavior is verified. Complex components have benchmark scenarios that verify behavior; visual changes have been inspected, or the lack of visual verification is explicitly reported. In the final response, distinguish builds, tests, native smoke tests, visual inspection, and measured performance.
