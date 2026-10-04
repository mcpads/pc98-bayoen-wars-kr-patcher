use std::collections::BTreeMap;

use serde::Serialize;

use super::super::interface_text::InterfaceTextEntryPatchReport;
use super::super::mad_system_text::MadSystemTextEntryPatchReport;
use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::super::shared_alert_window::SharedAlertWindowPatchReport;
use super::super::shared_text::SharedGaijiGlyph;
use super::super::spring_capture_window::SpringCaptureWindowPatchReport;
use super::super::spring_recovery_window::SpringRecoveryWindowPatchReport;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PatchedSystemComponentReport {
    pub storage_capacity: usize,
    pub packed_storage_bytes: usize,
    pub storage_headroom: usize,
    pub semantic_reference_count: usize,
    pub storage_reference_count: usize,
    pub machine_code_reference_count: usize,
    pub metadata_reference_count: usize,
    pub runtime_insert_reference_count: usize,
    pub entries: Vec<MadSystemTextEntryPatchReport>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct PatchedInterfaceComponentReport {
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
    pub entries: Vec<InterfaceTextEntryPatchReport>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct MadSystemInterfaceTextPatchReport {
    pub supported_source_sha256: String,
    pub translation_status: String,
    pub build_status: DevelopmentBuildStatus,
    pub font_profile: String,
    pub font_sha256: String,
    pub available_gaiji_slots: usize,
    pub used_gaiji_slots: usize,
    pub gaiji_headroom: usize,
    pub glyphs: Vec<SharedGaijiGlyph>,
    pub system: PatchedSystemComponentReport,
    pub interface: PatchedInterfaceComponentReport,
    pub writes: Vec<PayloadWriteReport>,
}

pub(crate) struct PatchedMadSystemInterfaceTextPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: MadSystemInterfaceTextPatchReport,
}
