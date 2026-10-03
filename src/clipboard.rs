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
