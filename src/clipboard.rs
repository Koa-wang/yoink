use anyhow::{Context, Result};
use arboard::Clipboard;

/// Read the current clipboard contents as text.
///
/// Returns `Ok(None)` when the clipboard is empty or does not contain text.
pub fn get_text() -> Result<Option<String>> {
    let mut cb = Clipboard::new().context("failed to open system clipboard")?;
    match cb.get_text() {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

/// Write text to the system clipboard.
pub fn set_text(text: &str) -> Result<()> {
    let mut cb = Clipboard::new().context("failed to open system clipboard")?;
    cb.set_text(text.to_string())
        .context("failed to write to system clipboard")?;
    Ok(())
}

/// A cheap clipboard change counter, when the platform provides one.
///
/// Returns `None` where no efficient counter is available, in which case the
/// daemon falls back to comparing clipboard text on every poll.
#[cfg(target_os = "macos")]
pub fn change_count() -> Option<u64> {
    use objc2_app_kit::NSPasteboard;
    Some(NSPasteboard::generalPasteboard().changeCount() as u64)
}

#[cfg(not(target_os = "macos"))]
pub fn change_count() -> Option<u64> {
    None
}
