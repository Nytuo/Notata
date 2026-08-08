use std::path::Path;

use rusty_chromaprint::{match_fingerprints, Configuration, Fingerprinter};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{NotataError, Result};

/// Chromaprint fingerprints beyond this length capture the whole gist of a
/// track; decoding further just costs time for no extra matching power.
const MAX_DECODE_SECONDS: u64 = 120;

/// Computes an acoustic fingerprint (Chromaprint-compatible) for a media
/// file by decoding it with symphonia and feeding the raw PCM to
/// rusty-chromaprint. Used to catch duplicates that share the same
/// recording but differ in container, bitrate, or encoding — unlike the
/// SHA-256 exact match, this looks at the actual audio content.
pub fn fingerprint_file(path: &Path) -> Result<Vec<u32>> {
    let file = std::fs::File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(NotataError::Audio)?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or_else(|| NotataError::Custom("No decodable audio track".to_string()))?
        .clone();
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(NotataError::Audio)?;

    let config = Configuration::preset_test2();
    let mut fingerprinter = Fingerprinter::new(&config);
    let mut started = false;
    let mut sample_buf: Option<SampleBuffer<i16>> = None;
    let mut decoded_frames: u64 = 0;
    let mut max_frames = u64::MAX;

    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(SymphoniaError::ResetRequired) => break,
            Err(e) => return Err(NotataError::Audio(e)),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let audio_buf = match decoder.decode(&packet) {
            Ok(buf) => buf,
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(e) => return Err(NotataError::Audio(e)),
        };

        if !started {
            let spec = *audio_buf.spec();
            fingerprinter
                .start(spec.rate, spec.channels.count() as u32)
                .map_err(|e| NotataError::Custom(e.to_string()))?;
            max_frames = spec.rate as u64 * MAX_DECODE_SECONDS;
            started = true;
        }

        // Packet frame counts can vary within a stream (VBR, AAC priming
        // frames, a shorter final packet), so the buffer is grown to fit
        // rather than sized once off the first packet — symphonia's
        // `copy_interleaved_ref` panics if capacity ever falls short.
        let needed = audio_buf.capacity() as u64;
        if sample_buf.as_ref().map(|b| b.capacity() as u64).unwrap_or(0) < needed {
            sample_buf = Some(SampleBuffer::<i16>::new(needed, *audio_buf.spec()));
        }

        if let Some(buf) = sample_buf.as_mut() {
            buf.copy_interleaved_ref(audio_buf);
            fingerprinter.consume(buf.samples());
        }

        decoded_frames += packet.dur();
        if decoded_frames >= max_frames {
            break;
        }
    }

    if !started {
        return Err(NotataError::Custom(
            "Could not decode any audio from the file".to_string(),
        ));
    }

    fingerprinter.finish();
    Ok(fingerprinter.fingerprint().to_vec())
}

/// Similarity between two fingerprints in the 0.0 (unrelated) – 1.0
/// (same recording) range, based on the matched segment coverage and how
/// tight those segments are.
pub fn similarity(fp1: &[u32], fp2: &[u32]) -> f64 {
    if fp1.is_empty() || fp2.is_empty() {
        return 0.0;
    }

    let config = Configuration::preset_test2();
    let segments = match match_fingerprints(fp1, fp2, &config) {
        Ok(segments) => segments,
        Err(_) => return 0.0,
    };

    if segments.is_empty() {
        return 0.0;
    }

    let total_items: usize = segments.iter().map(|s| s.items_count).sum();
    if total_items == 0 {
        return 0.0;
    }

    let weighted_score: f64 = segments
        .iter()
        .map(|s| s.score * s.items_count as f64)
        .sum::<f64>()
        / total_items as f64;

    let shorter_len = fp1.len().min(fp2.len()).max(1);
    let coverage = (total_items as f64 / shorter_len as f64).min(1.0);

    // `score` is a distance (0 = identical, up to 32 = unrelated).
    let tightness = (1.0 - weighted_score / 32.0).clamp(0.0, 1.0);

    coverage * tightness
}
