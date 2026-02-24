#[derive(Debug, Clone, Copy, PartialEq)]
pub enum State {
    Loading,
    Listening,
    Transcribing,
    Result,
    Error,
}
