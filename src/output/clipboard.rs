use wl_clipboard_rs::copy::{MimeType, Options, Source};

/// Copy text to the Wayland clipboard.
pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    let opts = Options::new();
    opts.copy(Source::Bytes(text.as_bytes().into()), MimeType::Text)?;
    Ok(())
}
