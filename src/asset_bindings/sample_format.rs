use anyhow::{Result, ensure};

const SHORT_HEADER_SIZE: usize = 2;
const LOOPED_HEADER_SIZE: usize = 6;
const LOOPED_HEADER_SUFFIX: [u8; 4] = [0x00, 0x00, 0x5a, 0x34];

#[derive(Debug)]
pub(crate) struct DecodedSample {
    pub header_size: usize,
    pub nibbles: Vec<u8>,
}

pub(crate) fn decode_sample(decoded: &[u8]) -> Result<DecodedSample> {
    ensure!(
        decoded.len() >= SHORT_HEADER_SIZE,
        "decoded SAMPA stream has no length header"
    );
    let payload_size = usize::from(u16::from_le_bytes([decoded[0], decoded[1]]));
    let header_size = decoded
        .len()
        .checked_sub(payload_size)
        .ok_or_else(|| anyhow::anyhow!("SAMPA declared payload exceeds its decoded stream"))?;
    ensure!(
        header_size == SHORT_HEADER_SIZE || header_size == LOOPED_HEADER_SIZE,
        "SAMPA stream has unsupported {header_size}-byte header"
    );
    if header_size == LOOPED_HEADER_SIZE {
        ensure!(
            decoded[SHORT_HEADER_SIZE..LOOPED_HEADER_SIZE] == LOOPED_HEADER_SUFFIX,
            "SAMPA extended header marker does not match"
        );
    }

    let mut nibbles = Vec::with_capacity(payload_size * 2);
    for byte in &decoded[header_size..] {
        nibbles.push(byte >> 4);
        nibbles.push(byte & 0x0f);
    }
    Ok(DecodedSample {
        header_size,
        nibbles,
    })
}

#[cfg(test)]
#[path = "sample_format_tests.rs"]
mod sample_format_tests;
