use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{stdout, Stdout, Write};

pub type Term = Terminal<CrosstermBackend<Stdout>>;

pub fn enter_tree() -> Result<Term> {
    enable_raw_mode()?;
    let _ = write!(stdout(), "\x1b[<u");
    let _ = stdout().flush();
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
    Ok(())
}

/// Switch from Helix back to tree mode, staying on the alternate screen.
pub fn back_to_tree() -> Result<Term> {
    disable_raw_mode()?;
    enable_raw_mode()?;
    reset_term();
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

pub fn resume_helix() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _ = write!(stdout, "\x1b[2J\x1b[H");
    let _ = stdout.flush();
    reset_term();
    Ok(())
}

pub fn disable_forward() -> Result<()> {
    reset_term();
    disable_raw_mode()?;
    Ok(())
}
