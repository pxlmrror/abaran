use crate::session::{self, PtySession};
use anyhow::Result;
use std::ffi::CString;
use std::path::{Path, PathBuf};

pub enum GituiAction {
    ToggleTree,
    Exited,
}

impl From<session::ToolAction> for GituiAction {
    fn from(a: session::ToolAction) -> Self {
        match a {
            session::ToolAction::Toggle | session::ToolAction::LaunchLazygit => GituiAction::ToggleTree,
            session::ToolAction::Exited => GituiAction::Exited,
        }
    }
}

pub struct Session {
    inner: PtySession,
}

impl Session {
    pub fn start(dir: &Path) -> Result<Self> {
        let cmd = CString::new("gitui").unwrap();
        let args: &[CString] = &[];
        let inner = PtySession::start(&cmd, args, Some(dir), vec![0x07, 0x0f], vec![])?;
        Ok(Session { inner })
    }

    pub fn forward_io(&mut self) -> Result<GituiAction> {
        self.inner.forward_io().map(Into::into)
    }

    pub fn resize(&self) -> Result<()> {
        self.inner.resize()
    }

    pub fn drain(&self) {
        self.inner.drain();
    }

    pub fn stop_child(&self) {
        self.inner.stop_child();
    }

    pub fn cont_child(&self) {
        self.inner.cont_child();
    }

    pub fn child_alive(&self) -> bool {
        self.inner.child_alive()
    }
}

pub fn find_git_root(cwd: &Path) -> Option<PathBuf> {
    let mut current = if cwd.is_dir() {
        cwd.to_path_buf()
    } else {
        cwd.parent()?.to_path_buf()
    };

    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        let parent = current.parent()?;
        current = parent.to_path_buf();
    }
}
