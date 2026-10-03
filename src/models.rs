/// A single clipboard history entry.
#[derive(Debug, Clone)]
pub struct Entry {
    pub id: i64,
    pub content: String,
    pub source: String,
    /// Original creation timestamp (kept in storage even though the UI
    /// primarily surfaces `updated_at`).
    #[allow(dead_code)]
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
}

impl Entry {
    /// Collapse the content onto a single line for terminal display.
    pub fn single_line(&self) -> String {
        let mut out = String::with_capacity(self.content.len());
        for c in self.content.chars() {
            match c {
                '\n' => out.push_str("⏎ "),
                '\r' => {}
                '\t' => out.push_str("  "),
                c if c.is_control() => {}
                c => out.push(c),
            }
        }
        out
    }

    /// Single-line preview truncated to `max` characters.
    pub fn preview(&self, max: usize) -> String {
        let line = self.single_line();
        truncate(&line, max)
    }
}

/// Truncate a string to `max` characters, appending an ellipsis if needed.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let t: String = s.chars().take(max).collect();
        format!("{t}…")
    } else {
        s.to_string()
    }
}

/// Format a unix timestamp as a short local date/time string.
pub fn format_time(ts: i64) -> String {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(ts, 0)
        .single()
        .map(|d| d.format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// A small fuzzy matcher: every character of `query` must appear, in order,
/// in `text` (case-insensitive). Returns a score (higher is better).
pub fn fuzzy_match(query: &str, text: &str) -> Option<i64> {
    if query.is_empty() {
        return Some(0);
    }

    let needle: Vec<char> = query.chars().flat_map(|c| c.to_lowercase()).collect();
    let haystack: Vec<char> = text.to_lowercase().chars().collect();

    let mut score: i64 = 0;
    let mut qi = 0usize;
    let mut prev: Option<usize> = None;
    let mut first: Option<usize> = None;

    for (i, &c) in haystack.iter().enumerate() {
        if qi < needle.len() && c == needle[qi] {
            match prev {
                Some(p) if i == p + 1 => score += 15, // consecutive match
                Some(_) => score += 5,
                None => {
                    first = Some(i);
                    score += 100; // first character match
                }
            }
            prev = Some(i);
            qi += 1;
            if qi == needle.len() {
                break;
            }
        }
    }

    if qi == needle.len() {
        let first = first.unwrap_or(0) as i64;
        let last = prev.unwrap_or(0) as i64;
        let gap_penalty = (last - first).saturating_sub(needle.len() as i64);
        Some(score - gap_penalty * 2 - first)
    } else {
        None
    }
}
