mod cli;
mod clipboard;
mod config;
mod daemon;
mod db;
mod models;
mod tui;

use anyhow::Result;
use clap::Parser;

use crate::models::format_time;

fn main() {
    // Behave like standard Unix tools when stdout is closed early (e.g. `| head`).
    #[cfg(unix)]
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    let cli = cli::Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("yoinker: error: {e:#}");
        std::process::exit(1);
    }
}

fn run(cli: cli::Cli) -> Result<()> {
    match cli.command {
        None | Some(cli::Command::Tui) => tui::run(),

        Some(cli::Command::Daemon { foreground }) => daemon::run(foreground),

        Some(cli::Command::Get) => match clipboard::get_text()? {
            Some(text) => {
                print!("{text}");
                if !text.ends_with('\n') {
                    println!();
                }
                Ok(())
            }
            None => anyhow::bail!("clipboard does not contain text"),
        },

        Some(cli::Command::Set { text }) => {
            clipboard::set_text(&text)?;
            eprintln!("yoinker: clipboard set ({} chars)", text.chars().count());
            Ok(())
        }

        Some(cli::Command::Search { query, limit }) => {
            let cfg = config::Config::load()?;
            let conn = db::init(&config::db_path())?;
            let entries = db::search(&conn, &query, limit.unwrap_or(cfg.max_entries))?;
            for e in &entries {
                println!("{}", display_line(e));
            }
            Ok(())
        }

        Some(cli::Command::Clear) => {
            let conn = db::init(&config::db_path())?;
            let n = db::clear(&conn)?;
            db::vacuum(&conn)?;
            println!("cleared {n} {}", if n == 1 { "entry" } else { "entries" });
            Ok(())
        }

        Some(cli::Command::Rm { id }) => {
            let conn = db::init(&config::db_path())?;
            let n = db::delete(&conn, id)?;
            if n == 0 {
                anyhow::bail!("no entry with id {id}");
            }
            println!("deleted entry {id}");
            Ok(())
        }

        Some(cli::Command::Pin { id, unpin }) => {
            let conn = db::init(&config::db_path())?;
            let n = db::set_pinned(&conn, id, !unpin)?;
            if n == 0 {
                anyhow::bail!("no entry with id {id}");
            }
            println!("{} entry {id}", if unpin { "unpinned" } else { "pinned" });
            Ok(())
        }

        Some(cli::Command::History { limit }) => {
            let cfg = config::Config::load()?;
            let conn = db::init(&config::db_path())?;
            let entries = db::list(&conn, limit.unwrap_or(cfg.max_entries))?;
            for e in &entries {
                println!("{}", display_line(e));
            }
            Ok(())
        }

        Some(cli::Command::Config { edit }) => {
            let cfg = config::Config::load()?;
            let path = config::config_path();
            if edit {
                let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
                let status = std::process::Command::new(&editor).arg(&path).status()?;
                if !status.success() {
                    anyhow::bail!("editor exited with {status}");
                }
            } else {
                println!("{}", cfg.to_toml());
                println!("# config file: {}", path.display());
            }
            Ok(())
        }
    }
}

fn display_line(e: &models::Entry) -> String {
    let pin = if e.pinned { "◆ " } else { "  " };
    format!(
        "{:>4}  {}  {}  {:<16}  {}",
        e.id,
        format_time(e.updated_at),
        pin,
        e.source,
        e.single_line()
    )
}
