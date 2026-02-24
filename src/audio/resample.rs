use audioadapter_buffers::direct::InterleavedSlice;
use rubato::{Fft, FixedSync, Resampler};

/// Resample f32 audio from 48kHz to 16kHz (ratio 1:3).
pub fn resample_48k_to_16k(input: &[f32]) -> Vec<f32> {
    let nbr_frames = input.len();
    let channels = 1;

    let mut resampler = Fft::<f32>::new(48000, 16000, nbr_frames, 1, channels, FixedSync::Input)
        .expect("failed to create resampler");

    let output_len = resampler.process_all_needed_output_len(nbr_frames);
    let mut outdata = vec![0.0f32; output_len];

    let input_adapter = InterleavedSlice::new(input, channels, nbr_frames)
        .expect("failed to create input adapter");
    let mut output_adapter = InterleavedSlice::new_mut(&mut outdata, channels, output_len)
        .expect("failed to create output adapter");

    let (_nbr_in, nbr_out) = resampler
        .process_all_into_buffer(&input_adapter, &mut output_adapter, nbr_frames, None)
        .expect("resample failed");

    outdata.truncate(nbr_out);
    outdata
}
