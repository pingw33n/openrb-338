//! The output encoders: WAV (byte-identical to the engine export's file), FLAC and Ogg Vorbis.

use std::error::Error;
use std::num::{NonZeroU8, NonZeroU32};

use flacenc::component::BitRepr;
use flacenc::error::Verify;
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

pub const SAMPLE_RATE: u32 = 44100;

/// The Ogg stream serial of every .ogg file orb writes ("rb", the project's first name; kept so
/// the files stay the same).
const OGG_SERIAL: i32 = 0x7262;

/// The container chosen by the output file's extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Wav,
    Flac,
    Ogg,
}

impl Format {
    pub fn from_extension(ext: Option<&str>) -> Option<Format> {
        match ext?.to_ascii_lowercase().as_str() {
            "wav" => Some(Format::Wav),
            "flac" => Some(Format::Flac),
            "ogg" => Some(Format::Ogg),
            _ => None,
        }
    }
}

/// Encodes interleaved 16-bit stereo samples.
pub fn encode(format: Format, samples: &[i16], quality: f32) -> Result<Vec<u8>, Box<dyn Error>> {
    match format {
        Format::Wav => Ok(orb_engine::frame::export::wav_bytes(samples)),
        Format::Flac => flac(samples),
        Format::Ogg => ogg(samples, quality),
    }
}

fn flac(samples: &[i16]) -> Result<Vec<u8>, Box<dyn Error>> {
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .map_err(|e| format!("FLAC encoder configuration: {e:?}"))?;
    let pcm: Vec<i32> = samples.iter().map(|&s| s as i32).collect();
    let source = flacenc::source::MemSource::from_samples(&pcm, 2, 16, SAMPLE_RATE as usize);
    let stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .map_err(|e| format!("FLAC encoding failed: {e:?}"))?;
    let mut sink = flacenc::bitsink::ByteSink::new();
    stream
        .write(&mut sink)
        .map_err(|e| format!("FLAC output failed: {e:?}"))?;
    Ok(sink.as_slice().to_vec())
}

fn ogg(samples: &[i16], quality: f32) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = Vec::new();
    // A fixed stream serial, so that the same samples give the same file.
    let mut enc = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(SAMPLE_RATE).unwrap(),
        NonZeroU8::new(2).unwrap(),
        &mut out,
        OGG_SERIAL,
    )
    .bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: quality,
    })
    .build()?;
    for chunk in samples.chunks(2 * 4096) {
        let l: Vec<f32> = chunk
            .iter()
            .step_by(2)
            .map(|&s| s as f32 / 32768.0)
            .collect();
        let r: Vec<f32> = chunk
            .iter()
            .skip(1)
            .step_by(2)
            .map(|&s| s as f32 / 32768.0)
            .collect();
        enc.encode_audio_block([&l[..], &r[..]])?;
    }
    enc.finish()?;
    Ok(out)
}
