use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::App;

pub fn handle_key(app: &mut App, key: KeyEvent) {
    // Ctrl-C / Ctrl-D / Ctrl-Q quit the TUI.
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('c') | KeyCode::Char('d') | KeyCode::Char('q') => {
                app.should_quit = true;
            }
            _ => {}
        }
        return;
    }

    // Ignore Alt/Super/Hyper/Meta combos so they are not mistaken for the
    // plain-character shortcuts below. Shift is allowed: uppercase letters
    // are reported with the Shift modifier set.
    if key
        .modifiers
        .intersects(KeyModifiers::ALT | KeyModifiers::SUPER | KeyModifiers::HYPER | KeyModifiers::META)
    {
        return;
    }
    if app.search_mode {
        match key.code {
            KeyCode::Esc => app.exit_search(false),
            KeyCode::Enter => app.exit_search(true),
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.update_filter();
                app.selected = 0;
                app.scroll_offset = 0;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.update_filter();
                app.selected = 0;
                app.scroll_offset = 0;
            }
            _ => {}
        }
        return;
    }

    // Any key other than `D` cancels the pending "clear all" confirmation.
    if !matches!(key.code, KeyCode::Char('D')) {
        app.confirm_clear = false;
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.move_down(),
        KeyCode::Char('k') | KeyCode::Up => app.move_up(),
        KeyCode::Char('g') => app.select(0),
        KeyCode::Char('G') => app.select_last(),
        KeyCode::Enter => app.copy_selected(),
        KeyCode::Char('y') => app.copy_and_exit(),
        KeyCode::Char('/') => {
            app.search_mode = true;
            app.message = None;
        }
        KeyCode::Char('d') => app.delete_selected(),
        KeyCode::Char('D') => app.confirm_or_clear(),
        KeyCode::Char('p') => app.toggle_pin(),
        KeyCode::Char('t') => app.toggle_theme(),
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => app.select_digit(c),
        _ => {}
    }
}
