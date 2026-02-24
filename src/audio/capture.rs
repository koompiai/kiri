use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::config::*;

pub struct AudioCapture {
    frames: Arc<Mutex<Vec<f32>>>,
    pub audio_level: Arc<Mutex<f32>>,
    stop: Arc<AtomicBool>,
}

impl AudioCapture {
    pub fn new() -> Self {
        Self {
            frames: Arc::new(Mutex::new(Vec::new())),
            audio_level: Arc::new(Mutex::new(0.0)),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Record until silence is detected after speech. Returns 48kHz f32 audio.
    pub fn record_with_silence(&self) -> anyhow::Result<Vec<f32>> {
        self.stop.store(false, Ordering::Relaxed);
        self.frames.lock().unwrap().clear();

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| anyhow::anyhow!("No input device found"))?;

        let config = cpal::StreamConfig {
            channels: CHANNELS,
            sample_rate: RECORD_RATE,
            buffer_size: cpal::BufferSize::Default,
        };

        let frames = self.frames.clone();
        let audio_level = self.audio_level.clone();
        let stop = self.stop.clone();
        let stop2 = self.stop.clone();

        let mut speech_detected = false;
        let mut silence_start: Option<Instant> = None;
        let mut speech_start: Option<Instant> = None;

        let stream = device.build_input_stream(
            &config,
            move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                frames.lock().unwrap().extend_from_slice(data);

                let rms = (data.iter().map(|&s| s * s).sum::<f32>() / data.len() as f32).sqrt();
                *audio_level.lock().unwrap() = rms;

                let now = Instant::now();

                if rms > SILENCE_THRESHOLD {
                    silence_start = None;
                    if !speech_detected {
                        speech_detected = true;
                        speech_start = Some(now);
                    }
                } else if speech_detected {
                    if let Some(start) = speech_start {
                        if now.duration_since(start).as_secs_f32() < SPEECH_MIN_DURATION {
                            return;
                        }
                    }
                    if silence_start.is_none() {
                        silence_start = Some(now);
                    } else if let Some(start) = silence_start {
                        if now.duration_since(start).as_secs_f32() >= SILENCE_DURATION {
                            stop.store(true, Ordering::Relaxed);
                        }
                    }
                }
            },
            move |err| {
                eprintln!("Audio stream error: {err}");
            },
            None,
        )?;

        stream.play()?;

        let start = Instant::now();
        while !stop2.load(Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if start.elapsed().as_secs_f32() >= MAX_DURATION {
                break;
            }
        }

        drop(stream);
        let audio = self.frames.lock().unwrap().clone();
        Ok(audio)
    }

    pub fn get_level(&self) -> f32 {
        *self.audio_level.lock().unwrap()
    }
}
