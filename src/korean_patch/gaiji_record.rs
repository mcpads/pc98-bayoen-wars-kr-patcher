use anyhow::{Result, ensure};

use super::font_rasterizer::GLYPH_BYTES;

pub(crate) const GAIJI_RECORD_SIZE: usize = 34;
const TARGET_RECORD_PREFIX: [u8; 2] = [0, 0];

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct GaijiRecord {
    prefix: [u8; 2],
    bitmap: [u8; GLYPH_BYTES],
}

impl GaijiRecord {
    pub(crate) fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() == GAIJI_RECORD_SIZE,
            "GAIJI record must be {GAIJI_RECORD_SIZE} bytes, got {}",
            bytes.len()
        );
        let mut prefix = [0_u8; 2];
        prefix.copy_from_slice(&bytes[..2]);
        let mut bitmap = [0_u8; GLYPH_BYTES];
        bitmap.copy_from_slice(&bytes[2..]);
        Ok(Self { prefix, bitmap })
    }

    pub(crate) fn from_bitmap(bitmap: [u8; GLYPH_BYTES]) -> Self {
        Self {
            prefix: TARGET_RECORD_PREFIX,
            bitmap,
        }
    }

    pub(crate) fn require_target_format(&self) -> Result<()> {
        ensure!(
            self.prefix == TARGET_RECORD_PREFIX,
            "GAIJI record prefix differs from the target program's 00 00 format"
        );
        Ok(())
    }

    pub(crate) fn to_bytes(&self) -> [u8; GAIJI_RECORD_SIZE] {
        let mut bytes = [0_u8; GAIJI_RECORD_SIZE];
        bytes[..2].copy_from_slice(&self.prefix);
        bytes[2..].copy_from_slice(&self.bitmap);
        bytes
    }
}

#[cfg(test)]
#[path = "gaiji_record_tests.rs"]
mod gaiji_record_tests;
