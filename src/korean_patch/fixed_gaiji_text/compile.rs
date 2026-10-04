use anyhow::Result;

use super::SEGMENT_ID;
use super::model::CompiledFixedGaijiText;
use super::pages::compile_slot;
use crate::game_data::{GaijiCatalog, MadProgramCatalog};
use crate::korean_patch::mad_system_interface_text::require_segment;
use crate::korean_patch::shared_text::compile_gaiji_bank;
use crate::translation_analysis::TranslationCorpus;
use crate::translation_drafts::TranslationSurface;

pub(in crate::korean_patch) fn compile_fixed_gaiji_text(
    gaiji_com: &[u8],
    mad: &MadProgramCatalog,
    gaiji: &GaijiCatalog,
    corpus: &TranslationCorpus,
) -> Result<CompiledFixedGaijiText> {
    let segment = require_segment(
        corpus,
        SEGMENT_ID,
        TranslationSurface::FixedGaijiText,
        mad.fixed_gaiji_text.slots.len(),
    )?;
    let lines = segment
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, &mad.gaiji_readiness, &lines)?;
    let slots = mad
        .fixed_gaiji_text
        .slots
        .iter()
        .zip(&segment.entries)
        .map(|(slot, draft)| compile_slot(slot, draft, &bank))
        .collect::<Result<Vec<_>>>()?;
    Ok(CompiledFixedGaijiText { bank, slots })
}
