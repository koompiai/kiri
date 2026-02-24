use std::path::PathBuf;

pub const RECORD_RATE: u32 = 48000;
pub const WHISPER_RATE: u32 = 16000;
pub const CHANNELS: u16 = 1;

pub const SILENCE_THRESHOLD: f32 = 0.015;
pub const SILENCE_DURATION: f32 = 2.5;
pub const SPEECH_MIN_DURATION: f32 = 0.5;
pub const MAX_DURATION: f32 = 120.0;
pub const DONE_TIMEOUT: f32 = 5.0;

pub fn notes_dir() -> PathBuf {
    dirs::home_dir().unwrap().join("kiri")
}

pub fn models_dir() -> PathBuf {
    dirs::data_dir().unwrap().join("kiri").join("models")
}

pub fn default_model_path() -> PathBuf {
    models_dir().join("ggml-medium.bin")
}
