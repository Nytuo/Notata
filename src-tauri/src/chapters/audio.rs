use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::error::{NotataError, Result};

pub fn decode_full_mono(path: &str) -> Result<(Vec<f32>, u32)> {
    let file = std::fs::File::open(Path::new(path))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = Path::new(path).extension().and_then(|e| e.to_str()) {
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

    let mut sample_rate: Option<u32> = None;
    let mut channels: usize = 1;
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut samples: Vec<f32> = Vec::new();

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

        if sample_rate.is_none() {
            let spec = *audio_buf.spec();
            sample_rate = Some(spec.rate);
            channels = spec.channels.count().max(1);
        }

        let needed = audio_buf.capacity() as u64;
        if sample_buf.as_ref().map(|b| b.capacity() as u64).unwrap_or(0) < needed {
            sample_buf = Some(SampleBuffer::<f32>::new(needed, *audio_buf.spec()));
        }

        if let Some(buf) = sample_buf.as_mut() {
            buf.copy_interleaved_ref(audio_buf);
            if channels == 1 {
                samples.extend_from_slice(buf.samples());
            } else {
                for frame in buf.samples().chunks_exact(channels) {
                    let sum: f32 = frame.iter().sum();
                    samples.push(sum / channels as f32);
                }
            }
        }
    }

    let sample_rate =
        sample_rate.ok_or_else(|| NotataError::Custom("Could not decode any audio from the file".to_string()))?;

    Ok((samples, sample_rate))
}
