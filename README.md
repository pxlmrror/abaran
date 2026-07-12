> [!CAUTION]
> Built entirely by AI — OpenCode ([Go](https://github.com/anomalyco/opencode))
> + DeepSeek v4 Pro, costing around $15 in API credits. My Rust knowledge
> begins and ends at `cargo run`. Use at your own risk. If your terminal
> catches fire, your files vanish, or Helix speaks in tongues — that's on you.

---

# abaran

**The shroud for Helix.**

[`abaran`](https://github.com/pxlmrror/abaran) wraps your editor with the
terminal tools you need day-to-day — a file tree, git interface, and
project-wide find-and-replace — all without leaving the terminal.

The name comes from the Bengali word **আবরণ** — a _shroud_ or _covering_.
abaran cloaks Helix behind a seamless TUI layer: you browse your project in
the tree, open files directly into Helix, toggle into gitui for git ops, or
drop into scooter for interactive find-and-replace. The PTY machinery that
pipes I/O between tools runs invisibly underneath.

> **This is a stopgap.** Once Helix ships a plugin system that handles
> file manager, git UI, project wide search and replace plugins,
> this project becomes obsolete. Until then, abaran is here to fill the gap.

> **No pull requests.** I can't responsibly review PRs — this codebase is
> entirely AI-generated and I have no way to verify that incoming changes
> aren't malicious. If you find a bug or want a feature, [open an
> issue](https://github.com/pxlmrror/abaran/issues) and I'll look into it.

---

## Features

- **File tree** — gitignore-aware, Nerd Font icons, lazy-loaded directories
- **File operations** — create, rename, delete (trash or rm -f), copy/paste
- **gitui** — toggle into [gitui](https://github.com/extrawurst/gitui) for
  staging, commits, and branch management (Ctrl+G)
- **scooter** — toggle into
  [scooter](https://github.com/thomasschafer/scooter) for interactive
  project-wide find-and-replace (Ctrl+S)
- **Seamless Helix integration** — files open in Helix running inside a PTY
  with zero-latency I/O forwarding
- **No tmux required** — all tools share a single terminal window

## Prerequisites

| Dependency | Required | Why |
|------------|----------|-----|
| **Linux** | Yes | abaran only runs on Linux |
| [Helix (`hx`)](https://helix-editor.com) | Yes | The editor abaran wraps |
| [gitui](https://github.com/extrawurst/gitui) | No | Git operations |
| [scooter](https://github.com/thomasschafer/scooter) | No | Find-and-replace |
| [gio](https://wiki.gnome.org/Projects/GLib) | No | Trash support (falls back to `rm -rf`) |
| Kitty-compatible terminal | Recommended | Ghostty, kitty, WezTerm, or Alacritty |
| [Nerd Font](https://www.nerdfonts.com) | Recommended | File tree icons

## Installation

### curl (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/pxlmrror/abaran/main/install.sh | bash
```

Installs the latest release binary to `~/.local/bin`. Supports x86_64 and
aarch64 Linux.

If `~/.local/bin` is not in your `PATH`, add this to your shell config:

```bash
export PATH="${HOME}/.local/bin:${PATH}"
```

### Build from source

```bash
git clone https://github.com/pxlmrror/abaran.git
cd abaran
cargo install --path .
```

## Usage

```bash
abaran              # open current directory
abaran ~/projects   # open a specific directory
```

### Keybindings

| Key | Action |
|-----|--------|
| `j` / `k` / `↑` / `↓` | Navigate tree |
| `h` / `l` / `←` / `→` | Collapse / expand directory |
| `Enter` | Toggle directory / open file in Helix |
| `Ctrl+O` | Toggle between tree and Helix |
| `Ctrl+G` | Toggle gitui |
| `Ctrl+S` | Toggle scooter |
| `Ctrl+Z` | Suspend to background (fg to resume) |
| `/` | Search (with `n`/`p` for next/prev) |
| `Esc` | Clear search / clear selection |
| `c` | Mark for copy |
| `m` | Mark for cut |
| `v` | Paste clipboard |
| `a` | Create file |
| `A` | Create directory |
| `r` | Rename |
| `d` | Delete (trash via gio) |
| `D` | Delete permanently |
| `g` | Jump prefix (`gg` top, `ge` bottom) |
| `z` | Scroll prefix (`zz` center, `zt` top, `zb` bottom) |
| `q` | Quit |
| `?` | Toggle help |

## How It Works

abaran creates a pseudo-terminal (PTY) via `nix::pty::openpty`, forks Helix
inside it, and stores the PTY master file descriptor. In **tree mode**,
[ratatui](https://ratatui.rs) renders the file tree on the alternate screen.
Pressing `Enter` writes `:open <path>` to the PTY master to open the selected
file. In **tool mode**, a `poll`-based I/O loop forwards keystrokes to the PTY
and PTY output to stdout, intercepting toggle keys inline. When you switch
back to the tree, Helix keeps running — its output is drained in a non-blocking
loop to prevent flow-control deadlocks.

## License

[MIT](LICENSE)
