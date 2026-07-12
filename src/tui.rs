use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use nix::{
    poll::{poll, PollFd, PollFlags},
    unistd::read,
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Stdout, Write};
use std::os::fd::BorrowedFd;

pub type Term = Terminal<CrosstermBackend<Stdout>>;

fn drain_stdin() {
    let mut fds = [PollFd::new(
        unsafe { BorrowedFd::borrow_raw(0) },
        PollFlags::POLLIN,
    )];
    if poll(&mut fds, 50u16).unwrap_or(0) > 0 {
        let mut buf = [0u8; 256];
        let _ = read(unsafe { BorrowedFd::borrow_raw(0) }, &mut buf);
    }
    loop {
        let mut fds = [PollFd::new(
            unsafe { BorrowedFd::borrow_raw(0) },
            PollFlags::POLLIN,
        )];
        if poll(&mut fds, 0u16).unwrap_or(0) == 0 {
            break;
        }
        let mut buf = [0u8; 256];
        let _ = read(unsafe { BorrowedFd::borrow_raw(0) }, &mut buf);
    }
}

pub fn enter_tree() -> Result<Term> {
    enable_raw_mode()?;
    let _ = write!(stdout(), "\x1b[<u");
    let _ = stdout().flush();
    drain_stdin();
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

fn reset_term() {
    let _ = write!(stdout(), "\x1b[<u");
    let _ = write!(stdout(), "\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l");
    let _ = stdout().flush();
}

/// Switch from tree mode to Helix without leaving the alternate screen.
pub fn enter_helix() -> Result<()> {
    disable_raw_mode()?;
    // Clear the alternate screen for Helix
    let _ = write!(stdout(), "\x1b[2J\x1b[H");
    enable_raw_mode()?;
    reset_term();
    drain_stdin();
    Ok(())
}

/// Switch from Helix back to tree mode, staying on the alternate screen.
pub fn back_to_tree() -> Result<Term> {
    disable_raw_mode()?;
    enable_raw_mode()?;
    reset_term();
    drain_stdin();
    let _ = write!(stdout(), "\x1b[2J\x1b[H");
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout))?)
}

pub fn leave_tree() -> Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    reset_term();
    disable_raw_mode()?;
    Ok(())
}

pub fn prepare_suspend() -> Result<()> {
    execute!(stdout(), LeaveAlternateScreen)?;
    reset_term();
    disable_raw_mode()?;
    Ok(())
}

pub fn resume_tool() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _ = stdout.flush();
    reset_term();
    drain_stdin();
    Ok(())
}

pub fn disable_forward() -> Result<()> {
    reset_term();
    disable_raw_mode()?;
    Ok(())
}
