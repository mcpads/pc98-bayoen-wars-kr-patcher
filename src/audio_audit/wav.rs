use anyhow::{Context, Result, ensure};

const RIFF_HEADER_SIZE: usize = 44;

pub(super) fn encode_mono_u8(nibbles: &[u8], sample_rate: u32) -> Result<Vec<u8>> {
    ensure!(sample_rate > 0, "WAV sample rate is zero");
    ensure!(
        nibbles.iter().all(|nibble| *nibble <= 0x0f),
        "sample contains a value wider than one nibble"
    );
    let data_size = u32::try_from(nibbles.len()).context("WAV sample data is too large")?;
    let riff_size = data_size
        .checked_add(36)
        .context("WAV RIFF size overflow")?;
    let mut output = Vec::with_capacity(RIFF_HEADER_SIZE + nibbles.len());
    output.extend_from_slice(b"RIFF");
    output.extend_from_slice(&riff_size.to_le_bytes());
    output.extend_from_slice(b"WAVEfmt ");
    output.extend_from_slice(&16u32.to_le_bytes());
    output.extend_from_slice(&1u16.to_le_bytes());
    output.extend_from_slice(&1u16.to_le_bytes());
    output.extend_from_slice(&sample_rate.to_le_bytes());
    output.extend_from_slice(&sample_rate.to_le_bytes());
    output.extend_from_slice(&1u16.to_le_bytes());
    output.extend_from_slice(&8u16.to_le_bytes());
    output.extend_from_slice(b"data");
    output.extend_from_slice(&data_size.to_le_bytes());
    output.extend(nibbles.iter().map(|nibble| nibble * 17));
    Ok(output)
}

#[cfg(test)]
#[path = "wav_tests.rs"]
mod wav_tests;
