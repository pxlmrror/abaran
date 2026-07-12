---
name: abaran-dev
description: Use when developing or debugging abaran, the Rust TUI file manager for Helix. Covers ratatui, PTY/terminal management, Helix integration, and common pitfalls.
---

# abaran Development

## Build Commands
- Build: `cargo build`
- Test: `cargo test`
- Run: `cargo run`

## Project Structure
```
src/
├── main.rs  — CLI entry, Tree↔Helix mode orchestration
├── app.rs   — State machine, event handling, mode transitions
├── tree.rs  — File tree with lazy-loading, ratatui rendering
├── helix.rs — PTY management, I/O forwarding, toggle detection
└── tui.rs   — Terminal mode transitions
```

## Dependencies
- **ratatui 0.30** + **crossterm 0.29** — TUI rendering
- **nix 0.31** (features: term, ioctl, poll, process, signal, fs, uio) — PTY,
  fork/exec, poll, signals
- **walkdir 2** — directory traversal
- **libc 0.2** — raw syscalls (dup2, ioctl TIOCSWINSZ, close)
- **clap 4** + **anyhow 1** — CLI parsing and error handling

## Data Flow

```
User Keyboard
     │
     ▼
┌────────────────┐   Tree Mode   ┌──────────────┐
│   crossterm     │ ────────────► │    ratatui    │
│   (stdin)       │               │  (alternate   │
└───────┬────────┘               │   screen)    │
        │                        └──────────────┘
        │ Helix Mode
        ▼
┌────────────────┐   forward_io()   ┌─────────────┐
│  handle_stdin   │ ──────────────► │   PTY Master │
│  (Ctrl+O/2xEsc) │                 │  ⇒ hx child  │
│  intercept     │ ◄────────────── │ (PTY Slave)  │
│                │   handle_pty()  │              │
└────────────────┘                 └─────────────┘
```

## Common Issues and Fixes

### Kitty Keyboard Protocol
Ghostty wraps keystrokes as escape sequences (Crtl+O → `\x1b[15;5u`).
- **Reset:** send `\x1b[<u` to stdout in `enable_forward()` and `enter_tree()`
- **Filter:** strip `\x1b[>...u` sequences from Helix output in `handle_pty()`

### PTY Flow Control (Deadlock)
When in tree mode, Helix keeps writing → PTY output buffer fills → Helix
blocks on write → `open_file()` deadlocks.
- **Fix:** always call `Session::drain()` (non-blocking poll+read) in tree mode

### Alternate Screen Corruption
Helix renders on the alternate screen. If `LeaveAlternateScreen` is called
during mode switch, Helix's output goes to the main screen and corrupts.
- **Fix:** use `enter_helix()` and `back_to_tree()` which clear in-place on
  the alternate screen; never leave it during mode transitions

### Mouse Reporting
Helix sends `\x1b[?1000h`, `\x1b[?1002h`, `\x1b[?1003h`, `\x1b[?1006h` which
enable mouse tracking on the real terminal.
- **Filter:** `handle_pty()` strips `\x1b[?...h` sequences with mouse params
- **Cleanup:** `reset_term()` sends disable sequences on mode transitions

### Double-close of PTY fds
`PtyMaster`/`PtySlave` from `nix::pty::openpty()` wrap `OwnedFd`. After
extracting raw fds with `as_raw_fd()`:
1. `std::mem::forget(result.master)` and `result.slave`
2. Create new `unsafe { OwnedFd::from_raw_fd(fd) }` wrappers

### Exit Detection
Helix's LSP children keep PTY slave open after Helix exits → `read(master)`
never returns 0.
- **Fix:** use PID-based `child_alive()` (calls `waitpid(WNOHANG)`) at the
  top of the `forward_io()` loop
