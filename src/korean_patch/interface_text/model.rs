use std::collections::BTreeMap;

use serde::Serialize;

use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_alert_window::{CompiledSharedAlertWindow, SharedAlertWindowPatchReport};
use super::super::shared_text::{CompiledGaijiBank, SharedGaijiGlyph};
use super::super::spring_capture_window::{
    CompiledSpringCaptureWindow, SpringCaptureWindowPatchReport,
};
use super::super::spring_recovery_window::{
    CompiledSpringRecoveryWindow, SpringRecoveryWindowPatchReport,
};

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextEntryPatchReport {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub byte_size: usize,
    pub line_count: usize,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub content_sha256: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct InterfaceTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub storage_capacity: usize,
    pub packed_storage_bytes: usize,
    pub storage_headroom: usize,
    pub reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub unreferenced_entry_ids: Vec<String>,
    pub shared_alert_window: SharedAlertWindowPatchReport,
    pub stage_result_window: SharedAlertWindowPatchReport,
    pub spring_capture_window: SpringCaptureWindowPatchReport,
    pub spring_recovery_window: SpringRecoveryWindowPatchReport,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub entries: Vec<InterfaceTextEntryPatchReport>,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedInterfaceTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: InterfaceTextPatchReport,
}

pub(in crate::korean_patch) struct CompiledInterfaceText {
    pub bank: CompiledGaijiBank,
    pub text_region_start: usize,
    pub text_region_end: usize,
    pub packed_storage_bytes: usize,
    pub populated_storage_bytes: usize,
    pub text_region_replacement: Vec<u8>,
    pub entries: Vec<CompiledInterfaceTextEntry>,
    pub shared_alert_window: CompiledSharedAlertWindow,
    pub stage_result_window: CompiledSharedAlertWindow,
    pub spring_capture_window: CompiledSpringCaptureWindow,
    pub spring_recovery_window: CompiledSpringRecoveryWindow,
}

#[derive(Debug)]
pub(in crate::korean_patch) struct CompiledInterfaceTextEntry {
    pub id: String,
    pub original_file_offset: usize,
    pub file_offset: usize,
    pub com_address: u16,
    pub lines: Vec<String>,
    pub line_screen_byte_widths: Vec<usize>,
    pub leading_attributes: Vec<u8>,
    pub trailing_attributes: Vec<u8>,
    pub bytes: Vec<u8>,
}
