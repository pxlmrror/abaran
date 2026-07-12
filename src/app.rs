use crate::{helix, tree, tui};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::path::PathBuf;

pub enum Action {
    Quit,
    SwitchToHelix,
    HelixExited,
    Continue,
}

pub struct App {
    pub tree: tree::FileTree,
    pub helix: Option<helix::Session>,
}

impl App {
    pub fn new(cwd: PathBuf) -> Result<Self> {
        let tree = tree::FileTree::new(cwd)?;
        Ok(App { tree, helix: None })
    }

    pub fn run_tree_mode(&mut self, terminal: &mut tui::Term) -> Result<Action> {
        loop {
            terminal.draw(|frame| {
                let area = frame.area();
                self.tree.render(frame, area);
            })?;

            let ev = event::read()?;
            if let Event::Key(key) = ev {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(Action::Quit),
                    KeyCode::Down | KeyCode::Char('j') => self.tree.navigate_down(),
                    KeyCode::Up | KeyCode::Char('k') => self.tree.navigate_up(),
                    KeyCode::Enter => {
                        // Enter on dir: toggle expand/collapse
                        if self.tree.is_selected_dir() {
                            self.tree.toggle_selected();
                        // Enter on file: open in Helix and switch to it
                        } else if let Some(path) = self.tree.selected_path() {
                            if let Some(ref mut session) = self.helix {
                                session.open_file(&path)?;
                            } else {
                                let session = helix::Session::start(&path)?;
                                self.helix = Some(session);
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
                        self.tree.collapse_selected();
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.tree.expand_selected();
                    }
                    _ => {}
                }
            }

            if let Some(ref session) = self.helix {
                // Drain PTY output so Helix doesn't block on writes
                session.drain();
                if !session.child_alive() {
                    self.helix = None;
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
            helix::HelixAction::Exited => {
                self.helix = None;
                Ok(Action::HelixExited)
            }
        }
    }

    pub fn cleanup(&mut self) {
        if let Some(session) = self.helix.take() {
            drop(session);
        }
    }
}
