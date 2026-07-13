use crate::session::{self, PtySession};
use anyhow::Result;
use std::ffi::CString;
use std::path::Path;

pub enum SerplAction {
    ToggleTree,
    Exited,
}

impl From<session::ToolAction> for SerplAction {
    fn from(a: session::ToolAction) -> Self {
        match a {
            session::ToolAction::Toggle
            | session::ToolAction::LaunchLazygit
            | session::ToolAction::LaunchSerpl => SerplAction::ToggleTree,
            session::ToolAction::Exited => SerplAction::Exited,
        }
    }
}

pub struct Session {
    inner: PtySession,
}

impl Session {
    pub fn start(dir: &Path) -> Result<Self> {
        let cmd = CString::new("serpl").unwrap();
        let args: &[CString] = &[];
        let inner = PtySession::start(
            &cmd,
            args,
            Some(dir),
            vec![0x13],
            vec![],
            vec![],
        )?;
        Ok(Session { inner })
    }

    pub fn forward_io(&mut self) -> Result<SerplAction> {
        self.inner.forward_io().map(Into::into)
    }

    pub fn resize(&mut self) -> Result<()> {
        self.inner.resize()
    }

    pub fn paint_screen(&self) -> Result<()> {
        self.inner.paint_screen()
    }

    pub fn drain(&mut self) {
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
