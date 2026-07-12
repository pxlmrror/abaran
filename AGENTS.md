# abaran Project Instructions

## Build & Test
- Build: `cargo build`
- Test: `cargo test` (5 unit tests for FileTree)
- Run: `cargo run`
- Lint: `cargo clippy`

## CI / Release
- `.github/workflows/ci.yml` — Runs `cargo build`, `cargo test`, `cargo clippy`,
  `cargo audit`, and `cargo deny` on push to `main`. Same checks you should
  run locally before pushing.
- `.github/workflows/bump-version.yml` — Manual `workflow_dispatch`. Bumps
  `Cargo.toml` version (patch/minor/major), commits, tags, and pushes. Pushing
  the tag triggers `release.yml`.
- `.github/workflows/release.yml` — Builds release binaries for x86_64 and
  aarch64 Linux when a `v*` tag is pushed. Uploads both artifacts to a GitHub
  release.
- `.github/workflows/codeql.yml` — CodeQL security analysis on push to `main`
  and weekly schedule.
- `install.sh` — Curl-able install script. Auto-detects arch, downloads the
  matching binary from the latest GitHub release, installs to `~/.local/bin`.

## Architecture
See `.opencode/skills/abaran-dev/SKILL.md` for detailed architecture, data
flow, bug history, and common pitfalls.

## Code Style
- Rust edition 2024
- No comments unless explicitly requested
- anyhow for error handling
- nix crate for Unix syscalls
- crossterm for TUI backend, raw nix read/write for tool I/O forwarding
- walkdir for directory traversal, ratatui for tree rendering
