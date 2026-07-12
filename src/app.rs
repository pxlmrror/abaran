use crate::{gitui, helix, ops, scooter, tree, tui};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use nix::sys::signal::{raise, Signal};
use ratatui::{
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use std::path::PathBuf;

pub enum Action {
    Quit,
    SwitchToHelix,
    HelixExited,
    SwitchToGitui,
    GituiExited,
    SwitchToScooter,
    ScooterExited,
    Continue,
}

enum InputState {
    CreateFile(PathBuf),
    CreateDir(PathBuf),
    Rename(PathBuf),
    ConfirmDelete(PathBuf),
    ConfirmForceDelete(PathBuf),
    Search,
}

pub struct App {
    pub tree: tree::FileTree,
    pub helix: Option<helix::Session>,
    pub gitui: Option<gitui::Session>,
    pub scooter: Option<scooter::Session>,
    clipboard: Vec<PathBuf>,
    clip_mode: Option<tree::ClipMode>,
    status: Option<String>,
    input: Option<InputState>,
    input_buffer: String,
    prefix: Option<char>,
    help_visible: bool,
}

impl App {
    pub fn new(cwd: PathBuf) -> Result<Self> {
        let tree = tree::FileTree::new(cwd)?;
        Ok(App {
            tree,
            helix: None,
            gitui: None,
            scooter: None,
            clipboard: Vec::new(),
            clip_mode: None,
            status: None,
            input: None,
            input_buffer: String::new(),
            prefix: None,
            help_visible: false,
        })
    }

    fn target_dir(&self) -> Option<PathBuf> {
        let path = self.tree.selected_path()?;
        if path.is_dir() {
            Some(path)
        } else {
            path.parent().map(|p| p.to_path_buf())
        }
    }

    fn reload_parent(&mut self, child_path: &std::path::Path) {
        if let Some(parent) = child_path.parent() {
            self.tree.reload_children(parent);
        } else {
            self.tree.reload_children(child_path);
        }
    }

    fn clear_input(&mut self) {
        self.input = None;
        self.input_buffer.clear();
    }

    fn execute_delete(&mut self, path: &std::path::Path) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match ops::delete(path) {
            Ok(()) => {
                self.status = Some(format!("Deleted {}", name));
                self.reload_parent(path);
            }
            Err(e) => {
                self.status = Some(format!("Delete failed: {}", e));
            }
        }
    }

    fn execute_force_delete(&mut self, path: &std::path::Path) {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match ops::force_delete(path) {
            Ok(()) => {
                self.status = Some(format!("Permanently deleted {}", name));
                self.reload_parent(path);
            }
            Err(e) => {
                self.status = Some(format!("Delete failed: {}", e));
            }
        }
    }

    fn execute_create_file(&mut self, dir: &std::path::Path, name: &str) {
        let new_path = dir.join(name);
        match ops::create_file(&new_path) {
            Ok(()) => {
                self.status = Some(format!("Created {}", name));
                self.tree.reload_children(dir);
            }
            Err(e) => {
                self.status = Some(format!("Create failed: {}", e));
            }
        }
    }

    fn execute_create_dir(&mut self, dir: &std::path::Path, name: &str) {
        let new_path = dir.join(name);
        match ops::create_dir(&new_path) {
            Ok(()) => {
                self.status = Some(format!("Created {}/", name));
                self.tree.reload_children(dir);
            }
            Err(e) => {
                self.status = Some(format!("Create failed: {}", e));
            }
        }
    }

    fn execute_rename(&mut self, src: &std::path::Path, new_name: &str) {
        let old_name = src
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if let Some(parent) = src.parent() {
            let dst = parent.join(new_name);
            match ops::rename_entry(src, &dst) {
                Ok(()) => {
                    self.status = Some(format!(
                        "Renamed {} -> {}",
                        old_name,
                        dst.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                    ));
                    self.reload_parent(src);
                }
                Err(e) => {
                    self.status = Some(format!("Rename failed: {}", e));
                }
            }
        }
    }

    fn try_launch_gitui(&mut self) -> Option<Action> {
        if let Some(session) = self.gitui.take() {
            drop(session);
        }
        let git_root = gitui::find_git_root(&self.tree.root.path)
            .unwrap_or_else(|| self.tree.root.path.clone());
        self.status = Some(format!("Launching gitui in {}", git_root.display()));
        match gitui::Session::start(&git_root) {
            Ok(session) => {
                self.gitui = Some(session);
                self.status = None;
                Some(Action::SwitchToGitui)
            }
            Err(e) => {
                self.status = Some(format!("Failed to start gitui: {}", e));
                None
            }
        }
    }

    fn try_launch_scooter(&mut self) -> Option<Action> {
        if let Some(session) = self.scooter.take() {
            drop(session);
        }
        let dir = self.target_dir().unwrap_or_else(|| self.tree.root.path.clone());
        self.status = Some(format!("Launching scooter in {}", dir.display()));
        match scooter::Session::start(&dir) {
            Ok(session) => {
                self.scooter = Some(session);
                self.status = None;
                Some(Action::SwitchToScooter)
            }
            Err(e) => {
                self.status = Some(format!("Failed to start scooter: {}", e));
                None
            }
        }
    }

    fn paste_clipboard(&mut self) {
        if self.clipboard.is_empty() {
            self.status = Some("Clipboard empty".into());
            return;
        }
        let mode = self.clip_mode;
        let target = match self.target_dir() {
            Some(t) => t,
            None => return,
        };
        let mut reload_dirs: Vec<std::path::PathBuf> = vec![target.clone()];
        if mode == Some(tree::ClipMode::Cut) {
            for src in &self.clipboard {
                if let Some(parent) = src.parent() {
                    reload_dirs.push(parent.to_path_buf());
                }
            }
        }
        let mut success = 0;
        let mut errors = 0;
        for src in std::mem::take(&mut self.clipboard) {
            let name = src.file_name().unwrap_or_default();
            let dst = target.join(name);
            let result = if mode == Some(tree::ClipMode::Cut) {
                ops::rename_entry(&src, &dst)
            } else {
                ops::copy_recursive(&src, &dst)
            };
            match result {
                Ok(()) => success += 1,
                Err(e) => {
                    errors += 1;
                    self.status = Some(format!("Paste failed: {}", e));
                }
            }
        }
        self.clip_mode = None;
        if success > 0 {
            let verb = if mode == Some(tree::ClipMode::Cut) {
                "Moved"
            } else {
                "Copied"
            };
            let extra = if errors > 0 {
                format!(" ({} errors)", errors)
            } else {
                String::new()
            };
            self.status = Some(format!("{} {} item(s){}", verb, success, extra));
        }
        for dir in &reload_dirs {
            self.tree.reload_children(dir);
        }
    }

    fn handle_input_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> bool {
        let state = match &self.input {
            Some(s) => s,
            None => return false,
        };

        match state {
            InputState::Search => match code {
                KeyCode::Esc => {
                    self.tree.clear_search();
                    self.clear_input();
                    self.status = None;
                }
                KeyCode::Enter => {
                    self.clear_input();
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                    self.tree.set_search(&self.input_buffer);
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input_buffer.push(c);
                    self.tree.set_search(&self.input_buffer);
                }
                _ => {}
            },
            InputState::ConfirmDelete(_) | InputState::ConfirmForceDelete(_) => match code {
                KeyCode::Char('y') => {
                    let taken = self.input.take();
                    match taken {
                        Some(InputState::ConfirmDelete(path)) => {
                            self.execute_delete(&path);
                        }
                        Some(InputState::ConfirmForceDelete(path)) => {
                            self.execute_force_delete(&path);
                        }
                        _ => {}
                    }
                }
                _ => {
                    self.status = Some("Delete cancelled".into());
                    self.clear_input();
                }
            },
            _ => match code {
                KeyCode::Esc => {
                    self.status = Some("Cancelled".into());
                    self.clear_input();
                }
                KeyCode::Enter => {
                    if self.input_buffer.is_empty() {
                        self.clear_input();
                        return true;
                    }
                    let prev = self.input.take();
                    let buf = std::mem::take(&mut self.input_buffer);
                    match prev {
                        Some(InputState::CreateFile(dir)) => {
                            self.execute_create_file(&dir, &buf);
                        }
                        Some(InputState::CreateDir(dir)) => {
                            self.execute_create_dir(&dir, &buf);
                        }
                        Some(InputState::Rename(src)) => {
                            self.execute_rename(&src, &buf);
                        }
                        _ => {}
                    }
                }
                KeyCode::Backspace => {
                    self.input_buffer.pop();
                }
                KeyCode::Char(c) if !modifiers.contains(KeyModifiers::CONTROL) => {
                    self.input_buffer.push(c);
                }
                _ => {}
            },
        }

        true
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let text = match &self.input {
            Some(InputState::Search) => {
                format!("/{}\u{2588}", self.input_buffer)
            }
            Some(InputState::CreateFile(dir)) => {
                format!(
                    "Create file in {}: {}\u{2588}",
                    dir.display(),
                    self.input_buffer
                )
            }
            Some(InputState::CreateDir(dir)) => {
                format!(
                    "Create directory in {}: {}\u{2588}",
                    dir.display(),
                    self.input_buffer
                )
            }
            Some(InputState::Rename(path)) => {
                let old = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                format!("Rename {}: {}\u{2588}", old, self.input_buffer)
            }
            Some(InputState::ConfirmDelete(path))
            | Some(InputState::ConfirmForceDelete(path)) => {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                format!("Delete {}? [y/n]", name)
            }
            None => self.status.clone().unwrap_or_default(),
        };

        let hint = Span::styled("? help", Style::default().fg(Color::DarkGray));
        let msg = Span::styled(text, Style::default().fg(Color::Yellow));
        let width = area.width as usize;

        let msg_len = msg.width();
        let hint_len = hint.width();
        let spacer = width.saturating_sub(msg_len + hint_len);

        let line = Line::from(vec![
            msg,
            Span::raw(" ".repeat(spacer)),
            hint,
        ]);
        frame.render_widget(Paragraph::new(line), area);
    }

    fn render_help(&self, frame: &mut Frame) {
        let area = centered_rect(52, 28, frame.area());
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Keybindings ");
        frame.render_widget(Clear, area);
        frame.render_widget(block, area);

        let help_text = vec![
            ("Navigation", vec![
                ("j / ↓", "Move down"),
                ("k / ↑", "Move up"),
                ("h / ←", "Collapse directory"),
                ("l / →", "Expand directory"),
                ("gg", "Jump to top"),
                ("ge", "Jump to bottom"),
                ("Enter", "Toggle dir / Open in Helix"),
            ]),
            ("File Operations", vec![
                ("c", "Mark for copy"),
                ("m", "Mark for cut"),
                ("v", "Paste"),
                ("d", "Delete (trash via gio)"),
                ("D", "Delete permanently"),
                ("a", "Create file"),
                ("A", "Create directory"),
                ("r", "Rename"),
            ]),
            ("Search", vec![
                ("/", "Enter search"),
                ("n", "Next match"),
                ("p", "Prev match"),
            ]),
            ("General", vec![
                ("Ctrl+O", "Toggle Helix"),
                ("Ctrl+G", "Toggle gitui"),
                ("Ctrl+S", "Toggle scooter"),
                ("Esc", "Clear search / selection"),
                ("q", "Quit"),
                ("?", "Toggle this help"),
            ]),
        ];

        let mut lines: Vec<Line<'_>> = Vec::new();
        for (section, items) in &help_text {
            lines.push(Line::from(
                Span::styled(
                    *section,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ));
            for (key, desc) in items {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {: <12}", key),
                        Style::default().fg(Color::Yellow),
                    ),
                    Span::raw(*desc),
                ]));
            }
        }

        let inner = area.inner(Margin {
            horizontal: 2,
            vertical: 1,
        });
        frame.render_widget(Paragraph::new(lines), inner);
    }

    pub fn run_tree_mode(&mut self, terminal: &mut tui::Term) -> Result<Action> {
        loop {
            terminal.draw(|frame| {
                let chunks =
                    Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                        .split(frame.area());
                self.tree.render(
                    frame,
                    chunks[0],
                    &self.clipboard,
                    self.clip_mode,
                );
                self.render_footer(frame, chunks[1]);
                if self.help_visible {
                    self.render_help(frame);
                }
            })?;

            let ev = event::read()?;
            if let Event::Key(key) = ev {
                if self.input.is_some() {
                    self.handle_input_key(key.code, key.modifiers);
                    continue;
                }

                if self.help_visible {
                    match key.code {
                        KeyCode::Char('?') | KeyCode::Esc | KeyCode::Enter => {
                            self.help_visible = false;
                        }
                        KeyCode::Char('q') => return Ok(Action::Quit),
                        _ => {}
                    }
                    continue;
                }

                if let Some(p) = self.prefix.take() {
                    match (p, key.code) {
                        ('g', KeyCode::Char('g')) => {
                            self.status = None;
                            self.tree.selected = 0;
                            continue;
                        }
                        ('g', KeyCode::Char('e')) => {
                            self.status = None;
                            let total = self.tree.visible_count();
                            self.tree.selected = total.saturating_sub(1);
                            continue;
                        }
                        ('z', KeyCode::Char('a')) => {
                            self.status = None;
                            if self.tree.any_expanded() {
                                self.tree.collapse_all();
                            } else {
                                self.tree.expand_all();
                            }
                            continue;
                        }
                        _ => {}
                    }
                }

                if key
                    .modifiers
                    .contains(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    continue;
                }
                match key.code {
                    KeyCode::Char('q') => return Ok(Action::Quit),
                    KeyCode::Char('?') => {
                        self.help_visible = !self.help_visible;
                    }
                    KeyCode::Char('g')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        self.status = None;
                        if let Some(action) = self.try_launch_gitui() {
                            return Ok(action);
                        }
                    }
                    KeyCode::Char('g') => {
                        self.status = None;
                        self.prefix = Some('g');
                    }
                    KeyCode::Char('s')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        self.status = None;
                        if let Some(action) = self.try_launch_scooter() {
                            return Ok(action);
                        }
                    }
                    KeyCode::Char('z')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        self.status = None;
                        if let Some(ref session) = self.helix {
                            session.stop_child();
                        }
                        if let Some(ref session) = self.gitui {
                            session.stop_child();
                        }
                        if let Some(ref session) = self.scooter {
                            session.stop_child();
                        }
                        tui::prepare_suspend()?;
                        raise(Signal::SIGTSTP)?;
                        if let Some(ref session) = self.helix {
                            session.cont_child();
                        }
                        if let Some(ref session) = self.gitui {
                            session.cont_child();
                        }
                        if let Some(ref session) = self.scooter {
                            session.cont_child();
                        }
                        *terminal = tui::enter_tree()?;
                        terminal.clear()?;
                    }
                    KeyCode::Char('z') => {
                        self.status = None;
                        self.prefix = Some('z');
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        self.status = None;
                        self.tree.navigate_down()
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        self.status = None;
                        self.tree.navigate_up()
                    }
                    KeyCode::Enter => {
                        self.status = None;
                        if self.tree.is_selected_dir()
                            && !self.tree.is_root_selected()
                        {
                            self.tree.toggle_selected();
                        } else if let Some(path) = self.tree.selected_path() {
                            if let Some(ref mut session) = self.helix {
                                session.open_file(&path)?;
                            } else {
                                match helix::Session::start(&path) {
                                    Ok(session) => self.helix = Some(session),
                                    Err(e) => {
                                        self.status = Some(format!("Failed to start Helix: {}", e));
                                        continue;
                                    }
                                }
                            }
                            return Ok(Action::SwitchToHelix);
                        }
                    }
                    KeyCode::Char('o') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        if self.helix.is_some() {
                            return Ok(Action::SwitchToHelix);
                        }
                    }
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.status = None;
                        if !self.tree.is_root_selected() {
                            self.tree.collapse_selected();
                        }
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.status = None;
                        if !self.tree.is_root_selected() {
                            self.tree.expand_selected();
                        }
                    }
                    KeyCode::Char('c') => {
                        self.status = None;
                        if let Some(path) = self.tree.selected_path() {
                            if self.clip_mode == Some(tree::ClipMode::Copy) {
                                if let Some(pos) =
                                    self.clipboard.iter().position(|p| p == &path)
                                {
                                    self.clipboard.remove(pos);
                                    if self.clipboard.is_empty() {
                                        self.clip_mode = None;
                                    }
                                    self.status =
                                        Some("Removed from clipboard".into());
                                } else {
                                    self.clipboard.push(path);
                                    self.status =
                                        Some("Added to clipboard".into());
                                }
                            } else {
                                self.clipboard.clear();
                                self.clipboard.push(path);
                                self.clip_mode = Some(tree::ClipMode::Copy);
                                self.status = Some("Copied to clipboard".into());
                            }
                        }
                    }
                    KeyCode::Char('m') => {
                        self.status = None;
                        if let Some(path) = self.tree.selected_path() {
                            if self.clip_mode == Some(tree::ClipMode::Cut) {
                                if let Some(pos) =
                                    self.clipboard.iter().position(|p| p == &path)
                                {
                                    self.clipboard.remove(pos);
                                    if self.clipboard.is_empty() {
                                        self.clip_mode = None;
                                    }
                                    self.status =
                                        Some("Removed from clipboard".into());
                                } else {
                                    self.clipboard.push(path);
                                    self.status =
                                        Some("Added to clipboard".into());
                                }
                            } else {
                                self.clipboard.clear();
                                self.clipboard.push(path);
                                self.clip_mode = Some(tree::ClipMode::Cut);
                                self.status = Some("Cut to clipboard".into());
                            }
                        }
                    }
                    KeyCode::Esc => {
                        if self.tree.is_searching() {
                            self.tree.clear_search();
                            self.status = None;
                        } else if !self.clipboard.is_empty() {
                            self.clipboard.clear();
                            self.clip_mode = None;
                            self.status = Some("Selection cleared".into());
                        }
                    }
                    KeyCode::Char('/') => {
                        self.status = None;
                        self.input_buffer.clear();
                        self.tree.clear_search();
                        self.input = Some(InputState::Search);
                    }
                    KeyCode::Char('n') => {
                        self.status = None;
                        self.tree.search_next();
                    }
                    KeyCode::Char('p') => {
                        self.status = None;
                        self.tree.search_prev();
                    }
                    KeyCode::Char('v') => {
                        self.status = None;
                        self.paste_clipboard();
                    }
                    KeyCode::Char('d') => {
                        self.status = None;
                        if let Some(path) = self.tree.selected_path() {
                            self.input = Some(InputState::ConfirmDelete(path));
                        }
                    }
                    KeyCode::Char('D') => {
                        self.status = None;
                        if let Some(path) = self.tree.selected_path() {
                            self.input =
                                Some(InputState::ConfirmForceDelete(path));
                        }
                    }
                    KeyCode::Char('a') if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                        self.status = None;
                        if let Some(dir) = self.target_dir() {
                            self.input = Some(InputState::CreateFile(dir));
                            self.input_buffer.clear();
                        }
                    }
                    KeyCode::Char('A') => {
                        self.status = None;
                        if let Some(dir) = self.target_dir() {
                            self.input = Some(InputState::CreateDir(dir));
                            self.input_buffer.clear();
                        }
                    }
                    KeyCode::Char('r') => {
                        self.status = None;
                        if let Some(path) = self.tree.selected_path() {
                            self.input_buffer = path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string();
                            self.input = Some(InputState::Rename(path));
                        }
                    }
                    _ => {}
                }
            }

            if let Some(ref session) = self.helix {
                session.drain();
                if !session.child_alive() {
                    self.helix = None;
                }
            }
            if let Some(ref session) = self.gitui {
                session.drain();
                if !session.child_alive() {
                    self.gitui = None;
                }
            }
            if let Some(ref session) = self.scooter {
                session.drain();
                if !session.child_alive() {
                    self.scooter = None;
                }
            }
        }
    }

    pub fn run_helix_mode(&mut self) -> Result<Action> {
        let session = match self.helix.as_mut() {
            Some(s) => s,
            None => return Ok(Action::Continue),
        };

        session.resize()?;
        match session.forward_io()? {
            helix::HelixAction::ToggleTree => Ok(Action::Continue),
            helix::HelixAction::LaunchLazygit => {
                self.status = None;
                Ok(self.try_launch_gitui().unwrap_or(Action::Continue))
            }
            helix::HelixAction::LaunchScooter => {
                self.status = None;
                Ok(self.try_launch_scooter().unwrap_or(Action::Continue))
            }
            helix::HelixAction::Exited => {
                self.helix = None;
                Ok(Action::HelixExited)
            }
        }
    }

    pub fn run_gitui_mode(&mut self) -> Result<Action> {
        let session = match self.gitui.as_mut() {
            Some(s) => s,
            None => return Ok(Action::Continue),
        };

        session.resize()?;
        match session.forward_io()? {
            gitui::GituiAction::ToggleTree => Ok(Action::Continue),
            gitui::GituiAction::Exited => {
                self.gitui = None;
                Ok(Action::GituiExited)
            }
        }
    }

    pub fn run_scooter_mode(&mut self) -> Result<Action> {
        let session = match self.scooter.as_mut() {
            Some(s) => s,
            None => return Ok(Action::Continue),
        };

        session.resize()?;
        match session.forward_io()? {
            scooter::ScooterAction::ToggleTree => Ok(Action::Continue),
            scooter::ScooterAction::Exited => {
                self.scooter = None;
                Ok(Action::ScooterExited)
            }
        }
    }

    pub fn cleanup(&mut self) {
        if let Some(session) = self.helix.take() {
            drop(session);
        }
        if let Some(session) = self.gitui.take() {
            drop(session);
        }
        if let Some(session) = self.scooter.take() {
            drop(session);
        }
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width.min(percent_x);
    let popup_height = r.height.min(percent_y);
    let x = r.x + (r.width.saturating_sub(popup_width)) / 2;
    let y = r.y + (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(x, y, popup_width, popup_height)
}
