use crate::tui;
use anyhow::{Context, Result};
use nix::{
    poll::{poll, PollFd, PollFlags},
    pty::openpty,
    sys::{
        signal::{kill, raise, Signal},
        wait::waitpid,
    },
    unistd::{execvp, fork, read, write, ForkResult, Pid},
};
use std::ffi::CString;
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, OwnedFd};
use std::path::Path;

pub enum ToolAction {
    Toggle,
    LaunchLazygit,
    LaunchScooter,
    Exited,
}

pub struct PtySession {
    master: OwnedFd,
    child_pid: Pid,
    pty_pending: Vec<u8>,
    toggle_bytes: Vec<u8>,
    launcher_bytes: Vec<u8>,
    scooter_launcher_bytes: Vec<u8>,
    stdin_pending: Vec<u8>,
    screen: vt100::Parser,
}

impl PtySession {
    pub fn start(
        command: &CString,
        args: &[CString],
        cwd: Option<&Path>,
        toggle_bytes: Vec<u8>,
        launcher_bytes: Vec<u8>,
        scooter_launcher_bytes: Vec<u8>,
    ) -> Result<Self> {
        let result = openpty(None, None).context("failed to create PTY")?;
        let master_fd = result.master.as_raw_fd();
        let slave_fd = result.slave.as_raw_fd();

        std::mem::forget(result.master);
        std::mem::forget(result.slave);

        let mut pipe_fds = [0i32; 2];
        if unsafe { libc::pipe(pipe_fds.as_mut_ptr()) } != 0 {
            unsafe {
                libc::close(master_fd);
                libc::close(slave_fd);
            }
            anyhow::bail!("pipe failed");
        }
        let (error_reader, error_writer) = (pipe_fds[0], pipe_fds[1]);

        let child = match unsafe { fork() }.context("fork failed")? {
            ForkResult::Child => {
                unsafe { libc::close(error_reader); }
                unsafe { libc::fcntl(error_writer, libc::F_SETFD, libc::FD_CLOEXEC); }
                unsafe { libc::close(master_fd); }
                if let Some(dir) = cwd {
                    unsafe { libc::chdir(dir.as_os_str().as_encoded_bytes().as_ptr() as *const libc::c_char); }
                }
                unsafe {
                    libc::dup2(slave_fd, 0);
                    libc::dup2(slave_fd, 1);
                    libc::dup2(slave_fd, 2);
                }
                if slave_fd > 2 {
                    unsafe { libc::close(slave_fd); }
                }
                execvp(command, args).ok();
                let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(1);
                unsafe {
                    libc::write(error_writer, &errno as *const i32 as *const libc::c_void, std::mem::size_of::<i32>());
                }
                std::process::exit(1);
            }
            ForkResult::Parent { child } => child,
        };

        unsafe {
            libc::close(slave_fd);
            libc::close(error_writer);
        }

        let err = {
            let mut errno = 0i32;
            let n = unsafe {
                libc::read(
                    error_reader,
                    &mut errno as *mut i32 as *mut libc::c_void,
                    std::mem::size_of::<i32>(),
                )
            };
            if n > 0 {
                Some(std::io::Error::from_raw_os_error(errno))
            } else {
                None
            }
        };
        unsafe { libc::close(error_reader); }

        if let Some(e) = err {
            unsafe { libc::close(master_fd); }
            anyhow::bail!(
                "{} not found: {}",
                command.to_string_lossy(),
                e
            );
        }

        let master = unsafe { OwnedFd::from_raw_fd(master_fd) };

        let (rows, cols) = terminal_size()
            .map(|ws| (ws.ws_row, ws.ws_col))
            .unwrap_or((24, 80));
        let screen = vt100::Parser::new(rows, cols, 0);

        if let Ok(ws) = terminal_size() {
            set_pty_size(master.as_raw_fd(), ws.ws_row, ws.ws_col).ok();
        }

        Ok(PtySession {
            master,
            child_pid: child,
            pty_pending: Vec::new(),
            toggle_bytes,
            launcher_bytes,
            scooter_launcher_bytes,
            stdin_pending: Vec::new(),
            screen,
        })
    }

    pub fn master_as_fd(&self) -> BorrowedFd<'_> {
        self.master.as_fd()
    }

    pub fn write_to_master(&self, data: &[u8]) -> Result<()> {
        let mut offset = 0;
        while offset < data.len() {
            let n = write(self.master.as_fd(), &data[offset..])
                .context("failed to write to PTY")?;
            offset += n;
        }
        Ok(())
    }

    pub fn forward_io(&mut self) -> Result<ToolAction> {
        loop {
            if !self.child_alive() {
                return Ok(ToolAction::Exited);
            }

            let (stdin_ready, pty_ready) = {
                let stdin_fd = unsafe { BorrowedFd::borrow_raw(0) };
                let mut fds = [
                    PollFd::new(stdin_fd, PollFlags::POLLIN),
                    PollFd::new(self.master.as_fd(), PollFlags::POLLIN),
                ];

                if poll(&mut fds, None::<u16>).context("poll failed")? == 0 {
                    continue;
                }

                (
                    fds[0]
                        .revents()
                        .unwrap_or(PollFlags::empty())
                        .contains(PollFlags::POLLIN),
                    fds[1]
                        .revents()
                        .unwrap_or(PollFlags::empty())
                        .contains(PollFlags::POLLIN),
                )
            };

            if stdin_ready
                && let Some(a) = self.handle_stdin()?
            {
                return Ok(a);
            }

            if pty_ready && self.handle_pty()? {
                return Ok(ToolAction::Exited);
            }
        }
    }

    fn handle_stdin(&mut self) -> Result<Option<ToolAction>> {
        let mut buf = [0u8; 4096];
        let n = read(unsafe { BorrowedFd::borrow_raw(0) }, &mut buf)
            .context("read from stdin failed")?;
        if n == 0 {
            return Ok(Some(ToolAction::Exited));
        }

        let data = if self.stdin_pending.is_empty() {
            buf[..n].to_vec()
        } else {
            self.stdin_pending.extend_from_slice(&buf[..n]);
            std::mem::take(&mut self.stdin_pending)
        };

        let mut filtered = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if data[i] != 0x1b || i + 2 >= data.len() || data[i + 1] != b'[' {
                filtered.push(data[i]);
                i += 1;
                continue;
            }

            let intermediary = data[i + 2];
            if intermediary != b'<' && intermediary != b'>' {
                filtered.push(data[i]);
                i += 1;
                continue;
            }

            let seq_start = i;
            i += 3;
            while i < data.len() && !(data[i] >= 0x40 && data[i] <= 0x7e) {
                i += 1;
            }
            if i >= data.len() {
                self.stdin_pending = data[seq_start..].to_vec();
                break;
            }

            let final_byte = data[i];
            i += 1;

            if final_byte != b'u' {
                filtered.extend_from_slice(&data[seq_start..i]);
            }
        }

        if self
            .scooter_launcher_bytes
            .iter()
            .any(|&b| filtered.contains(&b))
        {
            return Ok(Some(ToolAction::LaunchScooter));
        }

        if self
            .launcher_bytes
            .iter()
            .any(|&b| filtered.contains(&b))
        {
            return Ok(Some(ToolAction::LaunchLazygit));
        }

        if self.toggle_bytes.iter().any(|&b| filtered.contains(&b)) {
            return Ok(Some(ToolAction::Toggle));
        }

        if filtered.contains(&0x1a) {
            if let Some(pos) = filtered.iter().position(|&b| b == 0x1a)
                && pos > 0
            {
                write(self.master.as_fd(), &filtered[..pos])?;
            }
            self.suspend_session()?;
            return Ok(None);
        }

        write(self.master.as_fd(), &filtered)?;
        Ok(None)
    }

    fn handle_pty(&mut self) -> Result<bool> {
        let mut buf = [0u8; 4096];
        let n = read(self.master.as_fd(), &mut buf)
            .context("read from PTY failed")?;
        if n == 0 {
            return Ok(true);
        }

        let data = if self.pty_pending.is_empty() {
            buf[..n].to_vec()
        } else {
            self.pty_pending.extend_from_slice(&buf[..n]);
            std::mem::take(&mut self.pty_pending)
        };

        let mut out = Vec::with_capacity(data.len());
        let mut i = 0;

        while i < data.len() {
            if data[i] != 0x1b || i + 2 >= data.len() || data[i + 1] != b'[' {
                out.push(data[i]);
                i += 1;
                continue;
            }

            let intermediary = data[i + 2];
            let seq_start = i;

            if intermediary != b'>' && intermediary != b'?' {
                out.push(data[i]);
                i += 1;
                continue;
            }

            i += 3;
            let mut valid = true;
            while i < data.len() && !(data[i] >= 0x40 && data[i] <= 0x7e) {
                if data[i] != b';' && !data[i].is_ascii_digit() {
                    valid = false;
                }
                i += 1;
            }
            if i >= data.len() {
                self.pty_pending = data[seq_start..].to_vec();
                break;
            }

            let final_byte = data[i];
            i += 1;

            let drop = match (intermediary, valid, final_byte) {
                (b'>', true, b'u') => true,
                (b'?', true, b'h') => {
                    let params = &data[seq_start + 3..i - 1];
                    let s = std::str::from_utf8(params).unwrap_or("");
                    s.split(';').any(|p| matches!(p, "1000" | "1002" | "1003" | "1006"))
                }
                _ => false,
            };

            if !drop {
                out.extend_from_slice(&data[seq_start..i]);
            }
        }

        if !out.is_empty() {
            self.screen.process(&out);
            write(unsafe { BorrowedFd::borrow_raw(1) }, &out).ok();
        }
        Ok(false)
    }

    pub fn drain(&mut self) {
        loop {
            let mut fds = [PollFd::new(self.master.as_fd(), PollFlags::POLLIN)];
            if poll(&mut fds, 0u16).unwrap_or(0) == 0 {
                break;
            }
            let mut buf = [0u8; 4096];
            if let Ok(n) = read(self.master.as_fd(), &mut buf)
                && n > 0
            {
                self.screen.process(&buf[..n]);
            }
        }
    }

    pub fn resize(&mut self) -> Result<()> {
        if let Ok(ws) = terminal_size() {
            self.screen.set_size(ws.ws_row, ws.ws_col);
            set_pty_size(self.master.as_raw_fd(), ws.ws_row, ws.ws_col)
                .context("failed to resize PTY")?;
            let _ = kill(self.child_pid, Signal::SIGWINCH);
        }
        Ok(())
    }

    pub fn paint_screen(&self) -> Result<()> {
        use vt100::Color;

        let screen = self.screen.screen();
        let (rows, cols) = screen.size();

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"\x1b[H\x1b[2J");

        let mut curr_fg = Color::Default;
        let mut curr_bg = Color::Default;
        let mut curr_bold = false;
        let mut curr_italic = false;
        let mut curr_underline = false;
        let mut curr_inverse = false;

        for row in 0..rows {
            let pos = format!("\x1b[{};1H", row + 1);
            out.extend_from_slice(pos.as_bytes());

            for col in 0..cols {
                if let Some(cell) = screen.cell(row, col) {
                    let contents = cell.contents();

                    let fg = cell.fgcolor();
                    let bg = cell.bgcolor();
                    let bold = cell.bold();
                    let italic = cell.italic();
                    let underline = cell.underline();
                    let inverse = cell.inverse();

                    if fg != curr_fg || bg != curr_bg || bold != curr_bold
                        || italic != curr_italic || underline != curr_underline
                        || inverse != curr_inverse
                    {
                        write_sgr(&mut out, fg, bg, bold, italic, underline, inverse);
                        curr_fg = fg;
                        curr_bg = bg;
                        curr_bold = bold;
                        curr_italic = italic;
                        curr_underline = underline;
                        curr_inverse = inverse;
                    }

                    if contents.is_empty() {
                        out.push(b' ');
                    } else {
                        out.extend_from_slice(contents.as_bytes());
                    }
                } else {
                    out.push(b' ');
                }
            }
        }

        out.extend_from_slice(b"\x1b[?25l");
        write(unsafe { BorrowedFd::borrow_raw(1) }, &out)?;
        Ok(())
    }

    pub fn stop_child(&self) {
        let _ = kill(self.child_pid, Signal::SIGSTOP);
    }

    pub fn cont_child(&self) {
        let _ = kill(self.child_pid, Signal::SIGCONT);
    }

    fn suspend_session(&mut self) -> Result<()> {
        self.stop_child();
        tui::prepare_suspend()?;
        raise(Signal::SIGTSTP)?;
        tui::resume_tool()?;
        self.cont_child();
        self.resize()?;
        self.paint_screen()?;
        Ok(())
    }

    pub fn child_alive(&self) -> bool {
        matches!(
            waitpid(self.child_pid, Some(nix::sys::wait::WaitPidFlag::WNOHANG)),
            Ok(nix::sys::wait::WaitStatus::StillAlive)
        )
    }
}

impl Drop for PtySession {
    fn drop(&mut self) {
        let _ = kill(self.child_pid, Signal::SIGTERM);
    }
}

fn terminal_size() -> Result<libc::winsize> {
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(0, libc::TIOCGWINSZ, &mut ws) != 0 {
            anyhow::bail!("ioctl TIOCGWINSZ failed");
        }
        Ok(ws)
    }
}

fn set_pty_size(fd: std::os::raw::c_int, rows: u16, cols: u16) -> Result<()> {
    unsafe {
        let ws = libc::winsize {
            ws_row: rows,
            ws_col: cols,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        if libc::ioctl(fd, libc::TIOCSWINSZ, &ws) != 0 {
            anyhow::bail!("ioctl TIOCSWINSZ failed");
        }
    }
    Ok(())
}

fn write_sgr(
    out: &mut Vec<u8>,
    fg: vt100::Color,
    bg: vt100::Color,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
) {
    use vt100::Color;
    out.extend_from_slice(b"\x1b[0");
    if bold { out.extend_from_slice(b";1"); }
    if italic { out.extend_from_slice(b";3"); }
    if underline { out.extend_from_slice(b";4"); }
    if inverse { out.extend_from_slice(b";7"); }
    match fg {
        Color::Default => {}
        Color::Idx(i) => {
            let s = format!(";38;5;{}", i);
            out.extend_from_slice(s.as_bytes());
        }
        Color::Rgb(r, g, b) => {
            let s = format!(";38;2;{};{};{}", r, g, b);
            out.extend_from_slice(s.as_bytes());
        }
    }
    match bg {
        Color::Default => {}
        Color::Idx(i) => {
            let s = format!(";48;5;{}", i);
            out.extend_from_slice(s.as_bytes());
        }
        Color::Rgb(r, g, b) => {
            let s = format!(";48;2;{};{};{}", r, g, b);
            out.extend_from_slice(s.as_bytes());
        }
    }
    out.push(b'm');
}
