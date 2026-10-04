use super::super::font_rasterizer::FontProvenance;
use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::model::{
    CompiledFixedGaijiSlot, CompiledFixedGaijiText, FixedGaijiTextPatchReport,
    FixedGaijiTextSlotPatchReport,
};
use crate::source_disk::sha256_hex;

pub(in crate::korean_patch) fn fixed_gaiji_text_report(
    supported_source_sha256: String,
    font: FontProvenance,
    compiled: &CompiledFixedGaijiText,
    writes: Vec<PayloadWriteReport>,
) -> FixedGaijiTextPatchReport {
    FixedGaijiTextPatchReport {
        supported_source_sha256,
        translation_status: "needs_human_review".to_owned(),
        build_status: DevelopmentBuildStatus::DevelopmentOnly,
        font_profile: font.profile_id,
        font_sha256: font.font_sha256,
        available_gaiji_slots: compiled.bank.available_slot_count,
        used_gaiji_slots: compiled.bank.glyphs.len(),
        glyphs: compiled.bank.glyphs.clone(),
        slots: compiled.slots.iter().map(slot_report).collect(),
        writes,
    }
}

pub(super) fn slot_report(slot: &CompiledFixedGaijiSlot) -> FixedGaijiTextSlotPatchReport {
    FixedGaijiTextSlotPatchReport {
        id: slot.id.clone(),
        file_offset: slot.file_offset,
        visible_line_count: slot.lines.len(),
        occupied_cell_count: slot.lines.iter().map(|line| line.chars().count()).sum(),
        content_sha256: sha256_hex(&slot.bytes),
    }
}
