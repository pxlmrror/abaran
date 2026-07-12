---
name: abaran-dev
description: Use when developing or debugging abaran — the terminal cloak for Helix. Covers ratatui, PTY/terminal management, Helix/gitui/scooter integration, and common pitfalls.
---

# abaran Development

"আবরণ" (abaran) means "cloak" or "covering" in Bengali. The tool cloaks raw
PTY machinery behind a seamless TUI, wrapping Helix/gitui/scooter
transparently so the user navigates a file tree without seeing the terminal
juggling underneath.

## Build Commands
- Build: `cargo build`
- Test: `cargo test`
- Run: `cargo run`
- Lint: `cargo clippy`

## Project Structure
```
src/
├── main.rs    — CLI entry, Tree ↔ Helix/gitui/scooter mode orchestration
├── app.rs     — State machine, event handling, input/confirm dialogs, help, clipboard
├── tree.rs    — File tree with lazy-loading, gitignore-aware rendering, Nerd Font icons
├── helix.rs   — Helix-specific PTY session wrapper (toggle: Ctrl+O, double-Esc)
├── gitui.rs   — gitui-specific PTY session wrapper (toggle: Ctrl+G)
├── scooter.rs — scooter-specific PTY session wrapper (toggle: Ctrl+S)
├── session.rs — Generic PTY session: fork/exec, I/O forwarding, kitty filtering, suspend
├── tui.rs     — Terminal mode transitions, raw mode, alternate screen, stdin drain
└── ops.rs     — File operations: delete (gio trash or rm -f), copy, rename, create
```

## Dependencies
- **ratatui 0.30** + **crossterm 0.29** — TUI rendering and terminal I/O
- **nix 0.31** (features: term, ioctl, poll, process, signal, fs, uio) — PTY, fork/exec, poll, signals
- **walkdir 2** — directory traversal
- **ignore 0.4** — .gitignore pattern matching
- **libc 0.2** — raw syscalls (dup2, ioctl TIOCSWINSZ, close)
- **vt100 0.15** — ANSI terminal parser, maintains per-tool cell-grid screen buffer
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
        │ Tool Mode (Helix / gitui / scooter)
        ▼
┌────────────────┐   forward_io()   ┌─────────────┐
│  handle_stdin   │ ──────────────► │   PTY Master │
│  (toggle keys)  │                 │  ⇒ $tool     │
│  intercept     │ ◄────────────── │ (PTY Slave)  │
│                │   handle_pty()  │              │
└────────────────┘                 └─────────────┘
```

1. abaran creates a PTY pair via `nix::pty::openpty`
2. Forks; child execs the tool with PTY slave as stdio
3. Parent stores PTY master fd, communicates bidirectionally
4. **Tree mode**: ratatui renders file tree; Enter opens files via `:open` to PTY
5. **Tool mode**: polls stdin + PTY master, forwards bytes bidirectionally
6. **Toggle**: each tool has its own toggle keys registered in `PtySession::start`
7. **vt100 screen buffer**: `handle_pty()` and `drain()` feed PTY output into a
   `vt100::Parser` which maintains a cell-grid representation of the tool's
   display. `paint_screen()` iterates the cell grid and writes styled ANSI to
   stdout — this is how tool displays persist across mode switches and
   suspend/resume.

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

### 6. gitui/scooter blank after Ctrl+Z fg
**Root cause:** `EnterAlternateScreen` (`\x1b[?1049h`) clears the alt buffer on resume. The tool is SIGSTOP'd and resumed (SIGCONT), but doesn't redraw because no state changed — it's blocked on `read()` with SA_RESTART ignoring SIGWINCH. Sending input through the PTY also failed because the tool only emitted incremental ratatui diffs.
**Fix:** Added `vt100::Parser` screen buffer. `handle_pty()` and `drain()` feed all tool output to the parser, maintaining an up-to-date cell grid. On resume, `paint_screen()` writes the full cell grid (with SGR styling) to stdout, restoring the tool's exact pre-suspend display. No restart needed.

### 7. gitui/scooter session recreated on every toggle
**Root cause:** `try_launch_gitui()` and `try_launch_scooter()` always dropped the existing session and started a new one, even when already running.
**Fix:** Added early-return: if `self.gitui.is_some()`, return `Action::SwitchToGitui` directly. Combined with the screen buffer (bug 6 fix), toggle-back repaints the tool's display via `paint_screen()` without restarting.

## Common Issues and Fixes

### Kitty Keyboard Protocol
Ghostty wraps keystrokes as escape sequences (Ctrl+O → `\x1b[15;5u`).
- **Reset:** send `\x1b[<u` to stdout in `enable_forward()` and `enter_tree()`
- **Filter:** strip `\x1b[>...u` sequences from tool output in `handle_pty()`

### PTY Flow Control (Deadlock)
When in tree mode, the tool keeps writing → PTY output buffer fills → tool
blocks on write → `open_file()` deadlocks.
- **Fix:** always call `Session::drain()` (non-blocking poll+read) in tree mode

### Alternate Screen Corruption
Tools render on the alternate screen. If `LeaveAlternateScreen` is called
during mode switch, tool output goes to the main screen and corrupts.
- **Fix:** use `enter_helix()` and `back_to_tree()` which clear in-place on
  the alternate screen; never leave it during mode transitions

### Mouse Reporting
Tools may send `\x1b[?1000h`, `\x1b[?1002h`, `\x1b[?1003h`, `\x1b[?1006h` which
enable mouse tracking on the real terminal.
- **Filter:** `handle_pty()` strips `\x1b[?...h` sequences with mouse params
- **Cleanup:** `reset_term()` sends disable sequences on mode transitions

### Double-close of PTY fds
`PtyMaster`/`PtySlave` from `nix::pty::openpty()` wrap `OwnedFd`. After
extracting raw fds with `as_raw_fd()`:
1. `std::mem::forget(result.master)` and `result.slave`
2. Create new `unsafe { OwnedFd::from_raw_fd(fd) }` wrappers

### Exit Detection
LSP children (or subprocesses) keep PTY slave open after the main process
exits → `read(master)` never returns 0.
- **Fix:** use PID-based `child_alive()` (calls `waitpid(WNOHANG)`) at the
  top of the `forward_io()` loop

### Screen Buffer (vt100)
Each `PtySession` maintains a `vt100::Parser` that processes all PTY output
(filtered to remove Kitty/mouse sequences). The parser's cell grid mirrors
what the tool expects the terminal to look like.

- **Feed:** `handle_pty()` feeds filtered output (same bytes sent to stdout).
  `drain()` feeds raw PTY output (keeps buffer current when tool is backgrounded).
- **Resize:** `resize()` calls `screen.set_size()` to keep parser dimensions
  in sync with PTY.
- **Paint:** `paint_screen()` iterates all cells with their fg/bg colors and
  attributes, generates styled ANSI, and writes to stdout. Used on:
  * Toggle-back from tree/Helix to gitui/scooter (after `EnterAlternateScreen` clears screen)
  * Resume from Ctrl+Z (after `resume_tool()` clears screen)
- **Refresh lifecycle:** `drain()` (tree mode) keeps buffer current → toggle
  back → `resize()` syncs size → `paint_screen()` repaints → `forward_io()`
  resumes normal forwarding with the tool seeing a correct terminal state.
- **Important:** `drain()` must take `&mut self` to call `screen.process()`.
  All callers use `ref mut session` destructuring.
