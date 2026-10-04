use anyhow::{Result, ensure};

use super::CompiledGaijiBank;
use crate::game_data::{GRAPHIC_GLYPH_COUNT, GaijiCatalog, GaijiGlyphMeaning, parse_gaiji_program};

pub(crate) fn verify_gaiji_bank_readback(
    gaiji_com: &[u8],
    source: &GaijiCatalog,
    compiled: &CompiledGaijiBank,
) -> Result<()> {
    ensure!(
        gaiji_com == compiled.patched_program,
        "GAIJI program differs from the compiled shared-text bank"
    );
    verify_gaiji_bank_prefix_readback(gaiji_com, source, compiled)
}

pub(crate) fn verify_gaiji_bank_prefix_readback(
    gaiji_com: &[u8],
    source: &GaijiCatalog,
    compiled: &CompiledGaijiBank,
) -> Result<()> {
    let bank = gaiji_com
        .get(..compiled.patched_program.len())
        .ok_or_else(|| {
            anyhow::anyhow!("GAIJI program is shorter than the compiled shared-text bank")
        })?;
    ensure!(
        bank == compiled.patched_program,
        "GAIJI program prefix differs from the compiled shared-text bank"
    );
    let readback = parse_gaiji_program(bank)?;
    let graphic_slots = source
        .glyphs
        .iter()
        .zip(&readback.glyphs)
        .filter(|(expected, _)| matches!(expected.meaning, GaijiGlyphMeaning::Graphic { .. }))
        .collect::<Vec<_>>();
    ensure!(
        graphic_slots.len() == GRAPHIC_GLYPH_COUNT,
        "shared-text source changed the reserved graphic-slot population"
    );
    for (expected, actual) in graphic_slots {
        ensure!(
            matches!(actual.meaning, GaijiGlyphMeaning::Graphic { .. })
                && actual.sha256 == expected.sha256,
            "reserved graphic GAIJI slot {} changed",
            expected.index
        );
    }
    for index in &compiled.reserved_slot_indexes {
        let expected = source.glyphs.get(*index).ok_or_else(|| {
            anyhow::anyhow!("reserved GAIJI slot {index} left the source catalog")
        })?;
        let actual = readback.glyphs.get(*index).ok_or_else(|| {
            anyhow::anyhow!("reserved GAIJI slot {index} left the readback catalog")
        })?;
        ensure!(
            actual.sha256 == expected.sha256,
            "reserved GAIJI readiness slot {index} changed"
        );
    }
    Ok(())
}
