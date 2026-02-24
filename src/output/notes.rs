use std::fs;
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;

use crate::config::notes_dir;

/// Save transcribed text to a markdown file in ~/kiri/.
/// Returns the file path.
pub fn save_to_notes(text: &str) -> anyhow::Result<PathBuf> {
    let dir = notes_dir();
    fs::create_dir_all(&dir)?;

    let today = Local::now().format("%Y-%m-%d").to_string();
    let filepath = dir.join(format!("{today}.md"));
    let timestamp = Local::now().format("%H:%M").to_string();

    let is_new = !filepath.exists();
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&filepath)?;

    if is_new {
        writeln!(file, "# {today}\n")?;
    }
    writeln!(file, "<!-- {timestamp} -->\n{text}\n")?;

    Ok(filepath)
}
