use crate::session::{self, PtySession};
use anyhow::Result;
use std::ffi::CString;
use std::path::Path;

pub enum ScooterAction {
    ToggleTree,
    Exited,
}

impl From<session::ToolAction> for ScooterAction {
    fn from(a: session::ToolAction) -> Self {
        match a {
            session::ToolAction::Toggle
            | session::ToolAction::LaunchLazygit
            | session::ToolAction::LaunchScooter => ScooterAction::ToggleTree,
            session::ToolAction::Exited => ScooterAction::Exited,
        }
    }
}

pub struct Session {
    inner: PtySession,
}

impl Session {
    pub fn start(dir: &Path) -> Result<Self> {
        let cmd = CString::new("scooter").unwrap();
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

    pub fn forward_io(&mut self) -> Result<ScooterAction> {
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
