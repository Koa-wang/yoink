use anyhow::Result;
use rusqlite::Connection;

use crate::config::{self, Config};
use crate::db;
use crate::models::{fuzzy_match, Entry};

/// Actions that require side effects (e.g. clipboard access) and are
/// performed by the TUI event loop after the application state is updated.
pub enum Action {
    Copy { exit: bool },
}

pub struct App {
    pub config: Config,
    pub conn: Connection,
    pub entries: Vec<Entry>,
    pub filtered: Vec<Entry>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub search_mode: bool,
    pub search_query: String,
    pub message: Option<String>,
    pub should_quit: bool,
    pub confirm_clear: bool,
    pub pending_action: Option<Action>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config = Config::load()?;
        let conn = db::init(&config::db_path())?;
        db::cleanup(&conn, &config)?;
        let entries = db::list(&conn, config.max_entries)?;
        let filtered = entries.clone();
        Ok(Self {
            config,
            conn,
            entries,
            filtered,
            selected: 0,
            scroll_offset: 0,
            search_mode: false,
            search_query: String::new(),
            message: None,
            should_quit: false,
            confirm_clear: false,
            pending_action: None,
        })
    }

    pub fn refresh(&mut self) {
        self.entries = db::list(&self.conn, self.config.max_entries).unwrap_or_default();
        self.update_filter();
    }

    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered = self.entries.clone();
        } else {
            let mut matches: Vec<(i64, Entry)> = self
                .entries
                .iter()
                .filter_map(|e| fuzzy_match(&self.search_query, &e.content).map(|s| (s, e.clone())))
                .collect();
            matches.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| b.1.updated_at.cmp(&a.1.updated_at))
            });
            self.filtered = matches.into_iter().map(|(_, e)| e).collect();
        }
        self.clamp_selection();
    }

    pub fn clamp_selection(&mut self) {
        if self.filtered.is_empty() {
            self.selected = 0;
        } else if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len() - 1;
        }
    }

    pub fn exit_search(&mut self, keep: bool) {
        self.search_mode = false;
        if !keep {
            self.search_query.clear();
            self.update_filter();
        }
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn select(&mut self, idx: usize) {
        self.selected = idx.min(self.filtered.len().saturating_sub(1));
    }

    pub fn select_last(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.filtered.len() - 1;
        }
    }

    pub fn select_digit(&mut self, c: char) {
        if let Some(n) = c.to_digit(10) {
            self.select((n as usize).saturating_sub(1));
        }
    }

    pub fn selected_entry(&self) -> Option<&Entry> {
        self.filtered.get(self.selected)
    }

    pub fn selected_content(&self) -> Option<String> {
        self.selected_entry().map(|e| e.content.clone())
    }

    pub fn copy_selected(&mut self) {
        if self.selected_entry().is_some() {
            self.pending_action = Some(Action::Copy { exit: false });
        }
    }

    pub fn copy_and_exit(&mut self) {
        if self.selected_entry().is_some() {
            self.pending_action = Some(Action::Copy { exit: true });
        }
    }

    pub fn delete_selected(&mut self) {
        let id = self.selected_entry().map(|e| e.id);
        if let Some(id) = id {
            if db::delete(&self.conn, id).is_ok() {
                self.refresh();
                self.set_message("deleted");
            }
        }
    }

    pub fn clear_all(&mut self) {
        if db::clear(&self.conn).is_ok() {
            self.refresh();
            self.set_message("history cleared");
        }
    }

    pub fn toggle_pin(&mut self) {
        let target = self.selected_entry().map(|e| (e.id, !e.pinned));
        if let Some((id, pinned)) = target {
            if db::set_pinned(&self.conn, id, pinned).is_ok() {
                self.refresh();
                self.set_message(if pinned { "pinned" } else { "unpinned" });
            }
        }
    }

    pub fn confirm_or_clear(&mut self) {
        if self.confirm_clear {
            self.clear_all();
            self.confirm_clear = false;
        } else {
            self.confirm_clear = true;
            self.set_message("press D again to clear all history");
        }
    }

    pub fn set_message(&mut self, msg: &str) {
        self.message = Some(msg.to_string());
    }
}
