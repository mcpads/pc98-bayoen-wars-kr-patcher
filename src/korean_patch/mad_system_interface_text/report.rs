use super::super::font_rasterizer::FontProvenance;
use super::super::interface_text::{CompiledInterfaceText, interface_entry_report};
use super::super::mad_system_text::{CompiledMadSystemText, mad_system_entry_report};
use super::super::monochrome_text::DevelopmentBuildStatus;
use super::super::payload_writes::PayloadWriteReport;
use super::model::{
    MadSystemInterfaceTextPatchReport, PatchedInterfaceComponentReport,
    PatchedSystemComponentReport,
};
use crate::game_data::{MadProgramCatalog, MadSystemRuntimeCatalog};

pub(in crate::korean_patch) fn combined_report(
    supported_source_sha256: String,
    font: FontProvenance,
    mad: &MadProgramCatalog,
    system_runtime: &MadSystemRuntimeCatalog,
    system: &CompiledMadSystemText,
    interface: &CompiledInterfaceText,
    writes: Vec<PayloadWriteReport>,
) -> MadSystemInterfaceTextPatchReport {
    let system_storage_capacity = system.text_region_end - system.text_region_start;
    let interface_storage_capacity = interface.text_region_end - interface.text_region_start;
    MadSystemInterfaceTextPatchReport {
        supported_source_sha256,
        translation_status: "needs_human_review".to_owned(),
        build_status: DevelopmentBuildStatus::DevelopmentOnly,
        font_profile: font.profile_id,
        font_sha256: font.font_sha256,
        available_gaiji_slots: system.bank.available_slot_count,
        used_gaiji_slots: system.bank.glyphs.len(),
        gaiji_headroom: system.bank.available_slot_count - system.bank.glyphs.len(),
        glyphs: system.bank.glyphs.clone(),
        system: PatchedSystemComponentReport {
            storage_capacity: system_storage_capacity,
            packed_storage_bytes: system.packed_storage_bytes,
            storage_headroom: system_storage_capacity - system.packed_storage_bytes,
            semantic_reference_count: system_runtime.semantic_reference_count,
            storage_reference_count: system_runtime.storage_reference_count,
            machine_code_reference_count: system_runtime.machine_code_reference_count,
            metadata_reference_count: system_runtime.metadata_reference_count,
            runtime_insert_reference_count: system_runtime.runtime_insert_reference_count,
            entries: system
                .entries
                .iter()
                .map(|entry| mad_system_entry_report(entry, &system_runtime.references))
                .collect(),
        },
        interface: PatchedInterfaceComponentReport {
            storage_capacity: interface_storage_capacity,
            packed_storage_bytes: interface.packed_storage_bytes,
            storage_headroom: interface_storage_capacity - interface.packed_storage_bytes,
            reference_count: mad.interface_text.reference_catalog.reference_count,
            machine_code_reference_count: mad
                .interface_text
                .reference_catalog
                .machine_code_reference_count,
            metadata_reference_count: mad
                .interface_text
                .reference_catalog
                .metadata_reference_count,
            unreferenced_entry_ids: mad
                .interface_text
                .reference_catalog
                .unreferenced_entry_ids
                .clone(),
            stage_result_window: super::super::shared_alert_window::patch_report(
                &interface.stage_result_window,
            ),
            shared_alert_window: super::super::shared_alert_window::patch_report(
                &interface.shared_alert_window,
            ),
            spring_capture_window: super::super::spring_capture_window::patch_report(
                &interface.spring_capture_window,
            ),
            spring_recovery_window: super::super::spring_recovery_window::patch_report(
                &interface.spring_recovery_window,
            ),
            entries: interface
                .entries
                .iter()
                .map(|entry| {
                    interface_entry_report(entry, &mad.interface_text.reference_catalog.references)
                })
                .collect(),
        },
        writes,
    }
}
