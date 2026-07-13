use crate::session::{self, PtySession};
use anyhow::Result;
use nix::{
    poll::{poll, PollFd, PollFlags, PollTimeout},
    unistd::{read, write},
};
use std::ffi::CString;
use std::path::Path;

pub enum HelixAction {
    ToggleTree,
    LaunchLazygit,
    LaunchSerpl,
    Exited,
}

impl From<session::ToolAction> for HelixAction {
    fn from(a: session::ToolAction) -> Self {
        match a {
            session::ToolAction::Toggle => HelixAction::ToggleTree,
            session::ToolAction::LaunchLazygit => HelixAction::LaunchLazygit,
            session::ToolAction::LaunchSerpl => HelixAction::LaunchSerpl,
            session::ToolAction::Exited => HelixAction::Exited,
        }
    }
}

pub struct Session {
    inner: PtySession,
}

impl Session {
    pub fn start(file: &Path) -> Result<Self> {
        let cmd = CString::new("hx").unwrap();
        let file_cstr = CString::new(file.as_os_str().as_encoded_bytes())
            .expect("path contains null byte");
        let args = &[CString::new("hx").unwrap(), file_cstr];
        let inner = PtySession::start(&cmd, args, None, vec![0x0f], vec![0x07], vec![0x13])?;
        Ok(Session { inner })
    }

    pub fn open_file(&self, path: &Path) -> Result<()> {
        let _ = write(self.inner.master_as_fd(), b"\x1b\x1b\x1b");

        let mut drain_buf = [0u8; 4096];
        let mut fds = [PollFd::new(
            self.inner.master_as_fd(),
            PollFlags::POLLIN,
        )];
        if poll(&mut fds, PollTimeout::from(50u16)).unwrap_or(0) > 0 {
            let _ = read(self.inner.master_as_fd(), &mut drain_buf);
        }

        let cmd = format!(":open {}\r:redraw\r", path.display());
        self.inner.write_to_master(cmd.as_bytes())?;
        Ok(())
    }

    pub fn forward_io(&mut self) -> Result<HelixAction> {
        self.inner.forward_io().map(Into::into)
    }

    pub fn redraw(&self) -> Result<()> {
        self.inner.write_to_master(b":redraw\r")
    }

    pub fn resize(&mut self) -> Result<()> {
        self.inner.resize()
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
