use ndarray::Array2;
use rustfft::{num_complex::Complex32, FftPlanner};

use crate::error::{NotataError, Result};

use super::super::{audio, Chapter};

const HOP_SECONDS: f32 = 0.2;
const WINDOW_SECONDS: f32 = 0.4;
const KERNEL_HALF_SECONDS: f32 = 4.0;
const MIN_BOUNDARY_GAP_SECONDS: f32 = 8.0;

fn next_pow2(n: usize) -> usize {
    n.next_power_of_two()
}

const MIN_FREQ_HZ: f32 = 60.0;
const MAX_FREQ_HZ: f32 = 5000.0;

fn build_chromagram(samples: &[f32], sample_rate: u32) -> Array2<f32> {
    let sr = sample_rate as f32;
    let hop = ((HOP_SECONDS * sr) as usize).max(1);
    let window = ((WINDOW_SECONDS * sr) as usize).max(2);
    let fft_size = next_pow2(window);

    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_size);

    let n_frames = if samples.len() > window {
        (samples.len() - window) / hop + 1
    } else {
        1
    };

    let mut chroma = Array2::<f32>::zeros((n_frames.max(1), 12));
    let mut buffer = vec![Complex32::new(0.0, 0.0); fft_size];

    for frame_idx in 0..n_frames {
        let start = frame_idx * hop;
        let end = (start + window).min(samples.len());

        for v in buffer.iter_mut() {
            *v = Complex32::new(0.0, 0.0);
        }
        for (i, sample) in samples[start..end].iter().enumerate() {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / window as f32).cos();
            buffer[i] = Complex32::new(sample * w, 0.0);
        }

        fft.process(&mut buffer);

        let mut bins = [0.0f32; 12];
        for (bin, value) in buffer.iter().enumerate().take(fft_size / 2).skip(1) {
            let freq = bin as f32 * sr / fft_size as f32;
            if freq < MIN_FREQ_HZ || freq > MAX_FREQ_HZ {
                continue;
            }
            let magnitude = value.norm();
            let midi = 69.0 + 12.0 * (freq / 440.0).log2();
            let pitch_class = (midi.round() as i64).rem_euclid(12) as usize;
            bins[pitch_class] += magnitude;
        }

        let norm = bins.iter().map(|b| b * b).sum::<f32>().sqrt();
        if norm > 1e-6 {
            for b in bins.iter_mut() {
                *b /= norm;
            }
        }
        for (c, value) in bins.iter().enumerate() {
            chroma[[frame_idx, c]] = *value;
        }
    }

    chroma
}

fn self_similarity(chroma: &Array2<f32>) -> Array2<f32> {
    chroma.dot(&chroma.t())
}

fn novelty_curve(ssm: &Array2<f32>, kernel_half: usize) -> Vec<f32> {
    let n = ssm.shape()[0];
    let sigma = (kernel_half as f32 / 2.0).max(1.0);

    let size = kernel_half * 2;
    let mut kernel = vec![0.0f32; size * size];
    for da in 0..size {
        for db in 0..size {
            let a = da as f32 - kernel_half as f32;
            let b = db as f32 - kernel_half as f32;
            let sign = a.signum() * b.signum();
            if sign == 0.0 {
                continue;
            }
            let weight = (-(a * a + b * b) / (2.0 * sigma * sigma)).exp();
            kernel[da * size + db] = sign * weight;
        }
    }

    let mut novelty = vec![0.0f32; n];
    for t in kernel_half..n.saturating_sub(kernel_half) {
        let mut acc = 0.0f32;
        for da in 0..size {
            let i = t + da - kernel_half;
            for db in 0..size {
                let j = t + db - kernel_half;
                acc += ssm[[i, j]] * kernel[da * size + db];
            }
        }
        novelty[t] = acc;
    }
    novelty
}

fn pick_peaks(novelty: &[f32], min_gap: usize) -> Vec<usize> {
    if novelty.is_empty() {
        return Vec::new();
    }

    let mean = novelty.iter().sum::<f32>() / novelty.len() as f32;
    let variance = novelty.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / novelty.len() as f32;
    let threshold = mean + variance.sqrt() * 0.5;

    let mut candidates: Vec<(usize, f32)> = Vec::new();
    for i in 1..novelty.len() - 1 {
        if novelty[i] > threshold && novelty[i] >= novelty[i - 1] && novelty[i] >= novelty[i + 1] {
            candidates.push((i, novelty[i]));
        }
    }
    candidates.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    let mut chosen: Vec<usize> = Vec::new();
    for (idx, _) in candidates {
        if chosen.iter().all(|c: &usize| c.abs_diff(idx) >= min_gap) {
            chosen.push(idx);
        }
    }
    chosen.sort();
    chosen
}

pub fn detect(path: &str, mut on_progress: impl FnMut(&str, f64)) -> Result<Vec<Chapter>> {
    on_progress("decoding", 0.0);
    let (samples, sample_rate) = audio::decode_full_mono(path)?;
    if samples.is_empty() {
        return Err(NotataError::Custom("No audio samples decoded".to_string()));
    }
    let duration_ms = (samples.len() as f64 / sample_rate as f64 * 1000.0) as u64;

    on_progress("analyzing", 0.25);
    let chroma = build_chromagram(&samples, sample_rate);
    let n_frames = chroma.shape()[0];

    let hop = ((HOP_SECONDS * sample_rate as f32) as usize).max(1);
    let kernel_half = ((KERNEL_HALF_SECONDS / HOP_SECONDS) as usize).max(2);
    let min_gap = ((MIN_BOUNDARY_GAP_SECONDS / HOP_SECONDS) as usize).max(1);

    if n_frames <= kernel_half * 2 + 1 {
        on_progress("done", 1.0);
        return Ok(vec![Chapter {
            id: "chp1".to_string(),
            title: "Section 1".to_string(),
            start_ms: 0,
            end_ms: duration_ms,
        }]);
    }

    on_progress("comparing", 0.55);
    let ssm = self_similarity(&chroma);

    on_progress("finding boundaries", 0.8);
    let novelty = novelty_curve(&ssm, kernel_half);
    let peaks = pick_peaks(&novelty, min_gap);

    let mut boundary_ms: Vec<u64> = vec![0];
    for frame in peaks {
        boundary_ms.push((frame * hop) as u64 * 1000 / sample_rate as u64);
    }
    boundary_ms.push(duration_ms);
    boundary_ms.dedup();

    let mut chapters = Vec::new();
    for (i, window) in boundary_ms.windows(2).enumerate() {
        chapters.push(Chapter {
            id: format!("chp{}", i + 1),
            title: format!("Section {}", i + 1),
            start_ms: window[0],
            end_ms: window[1],
        });
    }

    on_progress("done", 1.0);
    Ok(chapters)
}
