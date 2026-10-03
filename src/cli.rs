use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "yoinker", version, about = "A terminal clipboard manager", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open the TUI interface (also the default with no subcommand)
    Tui,
    /// Start the clipboard watcher daemon
    Daemon {
        /// Run in the foreground instead of daemonizing
        #[arg(long)]
        foreground: bool,
    },
    /// Print the current clipboard contents to stdout
    Get,
    /// Set the clipboard contents
    Set {
        /// The text to copy to the clipboard
        text: String,
    },
    /// Search clipboard history (non-interactive)
    Search {
        /// Search query (fuzzy match)
        query: String,
        /// Maximum number of results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Delete all history entries
    Clear,
    /// Delete a single entry by id
    Rm {
        /// Entry id to delete
        id: i64,
    },
    /// Pin or unpin an entry by id
    Pin {
        /// Entry id
        id: i64,
        /// Unpin instead of pin
        #[arg(long)]
        unpin: bool,
    },
    /// Print clipboard history as plain text
    History {
        /// Maximum number of entries
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Show configuration (or open it in $EDITOR)
    Config {
        /// Open the config file in $EDITOR instead of printing it
        #[arg(long)]
        edit: bool,
    },
}
