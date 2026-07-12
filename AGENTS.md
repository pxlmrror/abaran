# abaran Project Instructions

## Build & Test
- Build: `cargo build`
- Test: `cargo test` (4 unit tests for FileTree)
- Run: `cargo run`
- Lint: `cargo clippy`

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
