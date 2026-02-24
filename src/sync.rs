use std::process::Command;

use crate::config::notes_dir;

fn git(args: &[&str]) -> anyhow::Result<std::process::Output> {
    let output = Command::new("git")
        .args(args)
        .current_dir(notes_dir())
        .output()?;
    Ok(output)
}

pub fn is_notes_repo() -> bool {
    notes_dir().join(".git").exists()
}

pub fn commit_notes() -> anyhow::Result<bool> {
    if !is_notes_repo() { return Ok(false); }
    git(&["add", "-A"])?;
    let status = git(&["diff", "--cached", "--quiet"])?;
    if status.status.success() { return Ok(false); }
    let msg = format!("kiri: notes update {}", chrono::Local::now().format("%Y-%m-%d %H:%M"));
    git(&["commit", "-m", &msg])?;
    Ok(true)
}

pub fn push_notes() -> anyhow::Result<()> {
    if !is_notes_repo() {
        anyhow::bail!("Not a git repo. Run: kiri sync --init <url>");
    }
    git(&["push", "-u", "origin", "main"])?;
    Ok(())
}

pub fn status() -> String {
    if !is_notes_repo() {
        return format!("Not a git repo: {}", notes_dir().display());
    }
    let log = git(&["log", "--oneline", "-5"]).ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    format!("Notes dir: {}\nRecent commits:\n{}", notes_dir().display(), log)
}
