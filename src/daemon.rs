use anyhow::Result;
use fs2::FileExt;

use crate::clipboard;
use crate::config::{self, Config};
use crate::db;

/// Start the daemon. On Unix, detach into the background unless `foreground`.
pub fn run(foreground: bool) -> Result<()> {
    let cfg = Config::load()?;

    if !foreground {
        daemonize_me()?;
    }

    run_inner(&cfg)
}

#[cfg(unix)]
fn daemonize_me() -> Result<()> {
    let dir = config::data_dir();
    std::fs::create_dir_all(&dir)?;
    let log = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("yoink.log"))?;
    let err = log.try_clone()?;
    daemonize::Daemonize::new()
        .working_directory("/")
        .stdout(log)
        .stderr(err)
        .start()
        .map_err(|e| anyhow::anyhow!("failed to daemonize: {e}"))?;
    Ok(())
}

#[cfg(not(unix))]
fn daemonize_me() -> Result<()> {
    eprintln!("yoink: background daemon mode is not supported on this platform; running in foreground");
    Ok(())
}

fn run_inner(cfg: &Config) -> Result<()> {
    let data_dir = config::data_dir();
    std::fs::create_dir_all(&data_dir)?;

    // Single-instance guard: keep the lock file open for the process lifetime.
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(data_dir.join("yoink.lock"))?;
    lock.try_lock_exclusive()
        .map_err(|_| anyhow::anyhow!("another yoink daemon is already running"))?;

    let conn = db::init(&config::db_path())?;
    let regexes = cfg.ignored_regexes();

    let mut last_content: Option<String> = None;
    let mut err_logged = false;
    let mut last_cleanup = chrono::Utc::now().timestamp();

    loop {
        let now = chrono::Utc::now().timestamp();
        if now - last_cleanup >= 60 {
            if let Err(e) = db::cleanup(&conn, cfg) {
                eprintln!("yoink daemon: cleanup failed: {e}");
            }
            last_cleanup = now;
        }

        match clipboard::get_text() {
            Ok(Some(text)) => {
                err_logged = false;
                if last_content.as_deref() != Some(text.as_str()) {
                    last_content = Some(text.clone());
                    if !text.trim().is_empty() && !is_ignored(&text, &regexes) {
                        let source = source_app().unwrap_or_else(|| "unknown".to_string());
                        if let Err(e) = db::insert_or_update(&conn, &text, &source) {
                            eprintln!("yoink daemon: failed to record clipboard: {e}");
                        }
                    }
                }
            }
            Ok(None) => {
                err_logged = false;
            }
            Err(e) => {
                if !err_logged {
                    eprintln!("yoink daemon: cannot read clipboard: {e}");
                    err_logged = true;
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(cfg.daemon.poll_interval_ms));
    }
}

fn is_ignored(text: &str, regexes: &[regex::Regex]) -> bool {
    regexes.iter().any(|r| r.is_match(text))
}

/// Best-effort detection of the frontmost application (the clipboard source).
#[cfg(target_os = "macos")]
fn source_app() -> Option<String> {
    let out = std::process::Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get name of first application process whose frontmost is true",
        ])
        .output()
        .ok()?;
    if out.status.success() {
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn source_app() -> Option<String> {
    let id = std::process::Command::new("xdotool")
        .arg("getactivewindow")
        .output()
        .ok()?;
    if !id.status.success() {
        return None;
    }
    let win = String::from_utf8_lossy(&id.stdout).trim().to_string();
    let out = std::process::Command::new("xprop")
        .args(["-id", &win, "WM_CLASS"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    // WM_CLASS(STRING) = "instance", "Class" — take the last quoted token.
    s.split('"')
        .rev()
        .find(|part| !part.trim().is_empty())
        .map(|part| part.to_string())
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn source_app() -> Option<String> {
    None
}
