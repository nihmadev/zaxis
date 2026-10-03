# Releasing zaxis

Cargo versions are numeric. Release display names append an adjective:
**0.0.1 Blinking**, with Git tag **v0.0.1**. Keep the display name in CHANGELOG.md
and README.md; Cargo.toml contains only `0.0.1`.

## First publication

1. Commit the prepared repository, push it to GitHub, and wait for Rust CI and
   Documentation to pass. Enable GitHub Pages with GitHub Actions as its source.
2. Confirm that the crate names `zaxis` and `zaxis-emoji` are available on crates.io,
   or that your account owns them. Log in to crates.io and authenticate locally with `cargo login`.
3. With a clean checkout, run:

   ```sh
   cargo +1.90.0 publish --workspace --dry-run --locked
   cargo +1.90.0 publish --workspace --locked
   ```

   Cargo publishes the dependency `zaxis-emoji` before `zaxis`. First publication is
   manual because Trusted Publishing requires an existing crate.
4. On crates.io, configure a GitHub Trusted Publisher for **both crates**, using
   repository `nihmadev/zaxis`, workflow `publish.yml`, and environment `release`.
   In GitHub, create the `release` environment
   and restrict it to version tags (`v*`). No permanent Cargo token is needed by CI.
5. Verify both crates.io pages and docs.rs builds, then create `v0.0.1` and a GitHub release
   titled `0.0.1 Blinking`, using its changelog entry as release notes.

   The workflow detects versions already uploaded manually and skips their registry
   upload, so creating the first GitHub release does not attempt a duplicate publication.

## Subsequent releases

1. Update Cargo.toml, regenerate Cargo.lock with `cargo check`, and update the changelog,
   README release name, security support statement, and guide version references.
   Bump `zaxis-emoji` only when its font data or API changes, and update the main
   crate's dependency requirement to match.
2. Commit and push the changes. Wait for CI to pass.
3. Create `v<version>` at that commit and publish a GitHub release titled
   `<version> <adjective>`. The Publish crate workflow reruns CI, checks that the
   tag matches Cargo.toml, verifies both packages, and publishes missing versions
   through Trusted Publishing in dependency order.

The workflow handles both GitHub prereleases and ordinary releases. A development
label is part of the display name; use a SemVer suffix only when you actually want
Cargo prerelease resolution. An already-uploaded version is checked but not uploaded again.

Before publishing, review `cargo package --workspace --list` and compressed package sizes.
The main crate includes library source, examples, tests, benchmarks, sample images,
Lato, licenses, and release/contribution documents. `zaxis-emoji` contains the
complete Noto Color Emoji font and its license. The website, node_modules, prompts,
and generated builds stay in the Git repository.
The compressed archive must stay below crates.io's default 10 MiB upload limit;
CI checks its size after packaging.

Reference: [crates.io Trusted Publishing](https://crates.io/docs/trusted-publishing).
