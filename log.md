# abaran Development Log

## Overview

abaran is a treemacs-like TUI file manager for the Helix editor. It runs as a CLI tool that opens a file tree, pipes selected files into Helix via PTY management, and allows toggling between the tree and Helix with keyboard shortcuts.

## Architecture

```
src/
├── main.rs    CLI entry, mode orchestration (Tree ↔ Helix)
├── app.rs     State machine, event handling, mode transitions
├── tree.rs    File tree with lazy-loading, ratatui rendering
├── helix.rs   PTY creation, Helix spawn, I/O forwarding, toggle
└── tui.rs     Terminal mode transitions (raw, alternate screen)
```

### Data Flow
1. abaran creates a PTY pair via `nix::pty::openpty`
2. Forks; child execs `hx` with PTY slave as stdio
3. Parent stores PTY master fd, communicates bidirectionally
4. **Tree mode**: ratatui renders file tree, Enter opens files via `:open` to PTY
5. **Helix mode**: polls stdin + PTY master, forwards bytes bidirectionally
6. **Toggle**: Ctrl+O (byte `0x0f`) or double-Escape in stdin handler

### Dependencies
| Crate | Purpose |
|-------|---------|
| ratatui 0.30 + crossterm 0.29 | TUI rendering and terminal I/O |
| nix 0.31 | PTY (term), fork/exec, poll, signals, ioctl |
| walkdir 2 | Directory traversal |
| clap 4 | CLI argument parsing |
| libc 0.2 | Raw syscalls (dup2, close, ioctl) |
| anyhow 1 | Error handling |

## Bug History

### 1. Alt+Enter toggle not working
**Root cause:** Ghostty uses Kitty keyboard protocol, encoding Alt+Enter as `\x1b[13;3u` instead of raw `\x1b\x0d`.
**Fix:** Switched to Ctrl+O (single byte `0x0f`) and double-Escape (`0x1b\x1b`) detection. Added `\x1b[<u` reset in `enable_forward()` and a `\x1b[>...u` filter in `handle_pty()`.

### 2. Second file open blocked (PTY flow control deadlock)
**Root cause:** When abaran is in tree mode, the PTY output buffer fills up with Helix's output. Helix blocks on write. When `open_file` tries to write `:open` to the PTY master, it also blocks because Helix can't read input.
**Fix:** Added `Session::drain()` — a non-blocking poll+read loop that empties the PTY output buffer. Called on every tree mode event loop iteration.

### 3. Subfolder files not opening (is_dir mis-detection)
**Root cause:** Modified `is_selected_dir` walk function had divergent traversal from `selected_path` (the `path` tracking vector changed the recursion behavior). Index-based walk found wrong entry.
**Fix:** Simplified `is_selected_dir` to use `selected_path()` for the path, then do a path-based tree search. Eliminates the index-based walk entirely.

### 4. `:redraw` needed after opening second file
**Root cause:** `leave_tree()` called `LeaveAlternateScreen`, putting the terminal on the main screen. Helix's output (designed for the alternate screen) rendered incorrectly.
**Fix:** Replaced `leave_tree() + enable_forward()` with `enter_helix()` — stays on alternate screen, just clears it. Sends `:redraw\r` after `:open` to force full re-render. Resized PTY + SIGWINCH on entering Helix mode.

### 5. Helix `:q` freeze (LSP keeps PTY open)
**Root cause:** When Helix exits, its LSP child processes keep the PTY slave open, so `read(master)` never returns 0. `forward_io` hangs forever.
**Fix:** Added `child_alive()` PID check at top of `forward_io` loop. Detects Helix exit via `waitpid(WNOHANG)` regardless of PTY state.

## Current Limitations
- No `.gitignore`-aware file filtering
- No file creation/deletion/rename operations
- Helix must be installed to function
- Only tested on Linux with Ghostty terminal
