# yoink

A terminal clipboard manager written in Rust. `yoink` watches the system
clipboard in the background, keeps a searchable history in SQLite, and lets you
quickly find and re-copy past snippets from a TUI.

## Features

- **Daemon** — polls the clipboard and records new text into SQLite, with
  de-duplication (re-copying bumps the timestamp instead of duplicating),
  ignore patterns, and automatic pruning.
- **TUI** — fuzzy search, keyboard navigation, copy-and-exit, delete, pin,
  clear, quick-select with `1-9`, and dark/light themes.
- **CLI** — scriptable `get` / `set` / `search` / `history` / `clear` /
  `config` commands.

## Install

```sh
cargo build --release
# binary at target/release/yoink
```

## Usage

```sh
yoink                    # open the TUI (default)
yoink tui                # same as above
yoink daemon             # start the background watcher (daemonizes on Unix)
yoink daemon --foreground
yoink get                # print current clipboard to stdout
yoink set "some text"    # write to the clipboard
yoink search "query"     # non-interactive fuzzy search
yoink history            # plain-text history list
yoink clear              # delete all history
yoink config             # show config (add --edit to open in $EDITOR)
```

### TUI keys

| Key | Action |
| --- | --- |
| `j` / `k` / `↑` / `↓` | move selection |
| `g` / `G` | jump to top / bottom |
| `Enter` | copy selected entry to clipboard |
| `y` | copy selected entry and quit |
| `/` | enter search mode |
| `Esc` (search) | cancel search |
| `Enter` (search) | accept search results |
| `d` | delete selected entry |
| `D` (twice) | clear all history |
| `p` | pin / unpin selected entry |
| `1`–`9` | select one of the first 9 entries |
| `q` / `Esc` / `Ctrl-C` / `Ctrl-D` / `Ctrl-Q` | quit |

## Data & config

- Config: `$XDG_CONFIG_HOME/yoink/config.toml`
  (macOS `~/Library/Application Support/yoink/config.toml`)
- Database: `$XDG_DATA_HOME/yoink/yoink.db`
  (macOS `~/Library/Application Support/yoink/yoink.db`)

```toml
max_entries = 1000            # maximum history entries
history_days = 30             # auto-delete entries older than this
ignored_patterns = []         # regexes; matching clipboard text is ignored
theme = "dark"                # "dark" or "light"
preview_length = 80           # preview truncation length in the TUI

[daemon]
poll_interval_ms = 500        # clipboard poll interval
```

Pinned entries are exempt from automatic cleanup.

## Platform notes

- macOS and Linux: `yoink daemon` detaches to the background.
- Windows: background daemon mode is not supported; use
  `yoink daemon --foreground`.
- Clipboard source-app detection is best-effort (falls back to `unknown`).
