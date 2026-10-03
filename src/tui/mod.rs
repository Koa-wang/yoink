use std::io;

use anyhow::Result;
use crossterm::{
    cursor,
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::clipboard;

mod app;
mod input;
mod ui;

pub use app::{Action, App};

pub fn run() -> Result<()> {
    let mut app = App::new()?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, cursor::Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, cursor::Show)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, &mut *app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Release {
                input::handle_key(&mut *app, key);
            }
        }

        if app.should_quit {
            break;
        }

        if let Some(action) = app.pending_action.take() {
            match action {
                Action::Copy { exit } => {
                    if let Some(content) = app.selected_content() {
                        clipboard::set_text(&content)?;
                        app.set_message("copied to clipboard");
                    }
                    if exit {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
