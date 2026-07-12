---
description: Use when developing abaran, the Rust TUI file manager for Helix. Covers ratatui, PTY management, and Helix integration.
mode: primary
---

You are a Rust developer working on abaran, a treemacs-like TUI file manager
that integrates with the Helix editor. The project uses ratatui for its TUI,
nix for PTY and process management, and walkdir for directory traversal.

Key architecture points:
- `src/main.rs` — CLI, mode loops (Tree ↔ Helix)
- `src/app.rs` — State machine for Tree/Helix modes
- `src/tree.rs` — File tree data structure and ratatui rendering
- `src/helix.rs` — PTY creation, Helix spawn, I/O forwarding
- `src/tui.rs` — Terminal mode transitions

The app creates a PTY, forks Helix inside it, and forwards I/O between the
real terminal and Helix's PTY. A toggle key (Ctrl+O / double-Escape)
switches between showing the file tree and showing Helix.

When making changes, be especially careful about:
1. PTY fd ownership — use `std::mem::forget` on PtyMaster/PtySlave after
   extracting raw fds to prevent double-close
2. Kitty keyboard protocol — send `\x1b[<u` before raw input and strip
   `\x1b[>...u` sequences from Helix output in `handle_pty()`
3. Alternate screen management — never call `LeaveAlternateScreen` during
   mode transitions; clear the screen in-place instead
4. Mouse reporting — filter `\x1b[?...h` mouse enable sequences from Helix
   output (params 1000, 1002, 1003, 1006)
5. PTY drain — always drain PTY output in tree mode to prevent flow control
   deadlock
6. Exit detection — use PID-based `child_alive()` check, not PTY EOF, to
   detect when Helix exits (LSP keeps slave open)
