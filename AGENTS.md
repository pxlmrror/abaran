# abaran Project Instructions

## Build & Test
- Build: `cargo build`
- Test: `cargo test` (4 unit tests for FileTree)
- Run: `cargo run`
- Lint: `cargo clippy`

## Architecture
See `log.md` for detailed architecture and bug history.

## Code Style
- Rust edition 2024
- No comments unless explicitly requested
- anyhow for error handling
- nix crate for Unix syscalls
- crossterm for TUI backend, raw nix read/write for Helix forwarding
- walkdir for directory traversal, ratatui List widget for tree rendering

## Critical Pitfalls

**Kitty keyboard protocol:** Ghostty wraps keystrokes in escape sequences
(`\x1b[15;5u` for Ctrl+O). Always reset with `\x1b[<u` before reading raw
input. Filter `\x1b[>...u` from Helix output in `handle_pty()`.

**PTY flow control:** Always call `drain()` on the PTY master in the tree
mode loop. Without it, Helix blocks on output and `open_file` deadlocks.

**Alternate screen:** Never call `LeaveAlternateScreen` during mode
transitions. Use `enter_helix()` and `back_to_tree()` which clear the screen
in-place on the alternate screen. Helix's rendering depends on this.

**OwnedFd ownership:** Extract raw fds from `nix::PtyMaster`/`PtySlave` with
`as_raw_fd()`, then `std::mem::forget()` the original before creating new
`OwnedFd::from_raw_fd()` wrappers.

**PTY zombie detection:** Helix's LSP children keep the PTY slave open after
Helix exits. Use `child_alive()` (PID-based `waitpid`) instead of PTY EOF to
detect exit.
