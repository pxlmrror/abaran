mod app;
mod helix;
mod gitui;
mod ops;
mod serpl;
mod session;
mod tree;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::io::{stdout, Write};
use std::path::PathBuf;

macro_rules! run_tool_from_helix {
    ($app:expr, $run_fn:ident) => {{
        write!(stdout(), "\x1b[2J\x1b[H")?;
        stdout().flush()?;
        loop {
            match $app.$run_fn()? {
                app::Action::Continue
                | app::Action::GituiExited
                | app::Action::SerplExited => break,
                app::Action::Quit => {
                    $app.cleanup();
                    tui::disable_forward()?;
                    return Ok(());
                }
                _ => {}
            }
        }
        if let Some(ref mut helix) = $app.helix {
            helix.drain();
            helix.redraw()?;
        }
    }};
}

macro_rules! run_tool_from_tree {
    ($app:expr, $terminal:expr, $run_fn:ident) => {{
        tui::enter_helix()?;
        loop {
            match $app.$run_fn()? {
                app::Action::Continue
                | app::Action::GituiExited
                | app::Action::SerplExited => {
                    $terminal = tui::back_to_tree()?;
                    break;
                }
                app::Action::Quit => {
                    $app.cleanup();
                    tui::disable_forward()?;
                    return Ok(());
                }
                _ => {}
            }
        }
    }};
}

#[derive(Parser)]
#[command(name = "abaran")]
struct Cli {
    #[arg(default_value = ".")]
    path: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::fs::canonicalize(&cli.path).unwrap_or(cli.path);

    let mut app = app::App::new(cwd)?;

    let mut terminal = tui::enter_tree()?;
    loop {
        match app.run_tree_mode(&mut terminal)? {
            app::Action::Quit => {
                app.cleanup();
                break;
            }
            app::Action::SwitchToHelix => {
                tui::enter_helix()?;

                loop {
                    match app.run_helix_mode()? {
                        app::Action::Continue => {
                            terminal = tui::back_to_tree()?;
                            break;
                        }
                        app::Action::HelixExited => {
                            terminal = tui::back_to_tree()?;
                            break;
                        }
                        app::Action::SwitchToGitui => {
                            run_tool_from_helix!(app, run_gitui_mode);
                        }
                        app::Action::SwitchToSerpl => {
                            run_tool_from_helix!(app, run_serpl_mode);
                        }
                        app::Action::Quit => {
                            app.cleanup();
                            tui::disable_forward()?;
                            return Ok(());
                        }
                        _ => {}
                    }
                }
            }
            app::Action::SwitchToGitui => {
                run_tool_from_tree!(app, terminal, run_gitui_mode);
            }
            app::Action::SwitchToSerpl => {
                run_tool_from_tree!(app, terminal, run_serpl_mode);
            }
            _ => {}
        }
    }

    tui::leave_tree()?;
    Ok(())
}
