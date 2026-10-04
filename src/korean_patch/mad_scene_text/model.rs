use std::collections::BTreeMap;

use serde::Serialize;

use super::super::baked_graphics::BakedGraphicsTextPatchReport;
use super::super::battle_callout_text::BattleCalloutPatchReport;
use super::super::dialogue_text::DialogueTextEntryPatchReport;
use super::super::fixed_gaiji_text::FixedGaijiTextPatchReport;
use super::super::mad_system_interface_text::{
    PatchedInterfaceComponentReport, PatchedSystemComponentReport,
};
use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::monochrome_text::MonochromeTextPatchReport;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_text::SharedGaijiGlyph;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PatchedDialogueComponentReport {
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub storage_capacity: usize,
    pub packed_storage_bytes: usize,
    pub storage_headroom: usize,
    pub group_count: usize,
    pub entry_count: usize,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub group_pointer_reference_count: usize,
    pub text_pointer_reference_count: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<DialogueTextEntryPatchReport>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSceneTransitionPatchReport {
    pub interface_bank_file_offset: usize,
    pub interface_extension_record_count: usize,
    pub interface_bank_record_count: usize,
    pub unit_status_bank_record_count: usize,
    pub dialogue_bank_file_offset: usize,
    pub dialogue_bank_byte_size: usize,
    pub dialogue_bank_record_count: usize,
    pub fixed_bank_file_offset: usize,
    pub fixed_bank_byte_size: usize,
    pub fixed_bank_record_count: usize,
    pub gaiji_record_count: usize,
    pub gaiji_file_original_size: usize,
    pub gaiji_file_output_size: usize,
    pub wrapper_file_offset: usize,
    pub wrapper_byte_size: usize,
    pub wrapper_storage_capacity: usize,
    pub wrapper_storage_headroom: usize,
    pub file_name_file_offset: usize,
    pub buffer_file_offset: usize,
    pub buffer_byte_size: usize,
    pub startup_entry_call_site_count: usize,
    pub call_site_count: usize,
    pub selection_entry_call_site_count: usize,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSceneTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub shared_available_gaiji_slots: usize,
    pub shared_used_gaiji_slots: usize,
    pub system_text_disposition: MadSystemTextDisposition,
    pub system: PatchedSystemComponentReport,
    pub interface: PatchedInterfaceComponentReport,
    pub dialogue: PatchedDialogueComponentReport,
    pub battle_callouts: BattleCalloutPatchReport,
    pub unit_list_status: super::super::unit_list_status::UnitListStatusPatchReport,
    pub fixed: FixedGaijiTextPatchReport,
    pub transition: MadSceneTransitionPatchReport,
    pub writes: Vec<PayloadWriteReport>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSceneNarrativePatchReport {
    pub scene_text: MadSceneTextPatchReport,
    pub monochrome: MonochromeTextPatchReport,
    pub baked: BakedGraphicsTextPatchReport,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedMadSceneTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: MadSceneTextPatchReport,
}

pub(crate) struct PatchedMadSceneNarrativePayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: MadSceneNarrativePatchReport,
}
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MadSystemTextDisposition {
    TranslatedDevelopment,
    PreservedSource,
}
