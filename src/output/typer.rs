use std::process::Command;

use crate::output::clipboard::copy_to_clipboard;

/// Paste text into the focused application.
/// Sets clipboard, then simulates Ctrl+Shift+V with ydotool.
pub fn paste_text(text: &str) -> anyhow::Result<()> {
    copy_to_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Ctrl+Shift+V: keycode 29=LCtrl, 42=LShift, 47=V
    Command::new("ydotool")
        .args(["key", "29:1", "42:1", "47:1", "47:0", "42:0", "29:0"])
        .status()
        .map_err(|e| anyhow::anyhow!("ydotool failed: {e}. Is ydotoold running?"))?;

    Ok(())
}
