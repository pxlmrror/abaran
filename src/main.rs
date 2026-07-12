mod app;
mod helix;
mod gitui;
mod ops;
mod session;
mod tree;
mod tui;

use anyhow::Result;
use clap::Parser;
use std::io::{stdout, Write};
use std::path::PathBuf;

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
                                write!(stdout(), "\x1b[2J\x1b[H")?;
                                stdout().flush()?;
                                loop {
                                    match app.run_gitui_mode()? {
                                        app::Action::Continue
                                        | app::Action::GituiExited => break,
                                        app::Action::Quit => {
                                            app.cleanup();
                                            tui::disable_forward()?;
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                                if let Some(ref helix) = app.helix {
                                    helix.drain();
                                    helix.redraw()?;
                                }
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
                    tui::enter_helix()?;

                    loop {
                        match app.run_gitui_mode()? {
                            app::Action::Continue => {
                                terminal = tui::back_to_tree()?;
                                break;
                            }
                            app::Action::GituiExited => {
                                terminal = tui::back_to_tree()?;
                                break;
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
                _ => {}
        }
    }

    tui::leave_tree()?;
    Ok(())
}
