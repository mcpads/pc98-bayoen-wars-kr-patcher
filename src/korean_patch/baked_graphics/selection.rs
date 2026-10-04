use anyhow::{Context, Result, ensure};

use super::canvas::{TextCanvas, write_compact_brgi, write_masked_brgi};
use crate::asset_bindings::BakedTextUnit;
use crate::korean_patch::font_catalog::FontTarget;
use crate::translation_drafts::TranslationDraftEntry;

const SEL3_DECODED_SIZE: usize = 0x9b40;

pub(super) fn compile_selection(
    source: &[u8],
    name_units: &[(&BakedTextUnit, &TranslationDraftEntry)],
    heading: (&BakedTextUnit, &TranslationDraftEntry),
) -> Result<Vec<u8>> {
    ensure!(
        source.len() == SEL3_DECODED_SIZE,
        "SEL3.DAT decoded size changed"
    );
    ensure!(
        name_units.len() == 10,
        "selection producer needs exactly ten character names"
    );
    let mut output = source.to_vec();
    for (binding, draft) in name_units {
        ensure!(
            binding.source_asset == "SEL3.DAT"
                && binding.producer_group_id == "selection-character-names"
                && binding.source_fragments.len() == 1
                && binding.screen_regions.len() == 1,
            "{} has an unexpected character-name producer binding",
            binding.id
        );
        let fragment = &binding.source_fragments[0];
        let region = &binding.screen_regions[0];
        ensure!(
            fragment.byte_size == region.width / 8 * region.height * 5,
            "{} masked record size changed",
            binding.id
        );
        let canvas = TextCanvas::render(
            FontTarget::BakedDisplay16,
            &draft.korean_text,
            region.width,
            region.height,
            1,
            true,
        )?;
        write_masked_brgi(&mut output, fragment.decoded_offset, &canvas, 0, 15)?;
    }

    let (binding, draft) = heading;
    ensure!(
        binding.id == "stage-select-heading"
            && binding.source_asset == "SEL3.DAT"
            && binding.producer_group_id == "selection-stage-heading"
            && binding.source_fragments.len() == 2
            && binding.screen_regions.len() == 2,
        "stage heading has an unexpected producer binding"
    );
    for fragment in &binding.source_fragments {
        let region = binding
            .screen_regions
            .iter()
            .find(|region| region.variant == fragment.role)
            .with_context(|| format!("heading fragment {} has no screen region", fragment.role))?;
        ensure!(
            fragment.byte_size == region.width / 8 * region.height * 4,
            "heading fragment {} size changed",
            fragment.role
        );
        let canvas = TextCanvas::render(
            FontTarget::BakedDisplay16,
            &draft.korean_text,
            region.width,
            region.height,
            2,
            false,
        )?;
        write_compact_brgi(&mut output, fragment.decoded_offset, &canvas, 0, 15)?;
    }
    Ok(output)
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;
