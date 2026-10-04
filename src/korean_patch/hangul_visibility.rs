use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{ExpectedWrite, ImageRegion, RegionKind, WriteIntent, WritePlan};
use serde::Serialize;

use super::font_rasterizer::{font_provenance, rasterize_hangul_syllable};
use super::gaiji_code_usage::require_unreferenced_gaiji_code;
use super::gaiji_record::{GAIJI_RECORD_SIZE, GaijiRecord};
use super::payload_writes::{PayloadFileWritePlan, PayloadWriteReport, apply_payload_write_plans};
use crate::game_data::parse_gaiji_program;

const GAIJI_FILE: &str = "GAIJI.COM";
const DIALOGUE_FILE: &str = "MAD.COM";
const DEMONSTRATION_CHARACTER: char = '가';
const DEMONSTRATION_GAIJI_SLOT: usize = 104;
const DEMONSTRATION_GAIJI_RECORD_OFFSET: usize = 0x0fba;
const DEMONSTRATION_CHARACTER_CODE: u16 = 0x772b;
const DEMONSTRATION_SHIFT_JIS_CODE: u16 = 0xec4a;
const DEMONSTRATION_INTERFACE_OFFSET: usize = 0x77e6;
const EXPECTED_INTERFACE_CHARACTER: [u8; 2] = [0xeb, 0xbb];
const DEMONSTRATION_DIALOGUE_OFFSET: usize = 0xb1bd;
const EXPECTED_DIALOGUE_CHARACTER: [u8; 2] = [0x82, 0xf1];
const EXPECTED_GAIJI_RECORD: [u8; GAIJI_RECORD_SIZE] = [
    0x00, 0x00, 0x00, 0x00, 0x18, 0x00, 0x18, 0x00, 0x18, 0x00, 0x18, 0xfc, 0x1f, 0x86, 0xf8, 0x06,
    0x18, 0x0c, 0x18, 0x18, 0x18, 0x30, 0x18, 0xe0, 0x18, 0x00, 0x18, 0x00, 0x18, 0x00, 0x18, 0x00,
    0x0f, 0xfc,
];

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct HangulVisibilityPatchReport {
    pub character: String,
    pub gaiji_slot: usize,
    pub character_code: String,
    pub shift_jis_code: String,
    pub interface_offset: usize,
    pub dialogue_file: String,
    pub dialogue_offset: usize,
    pub audited_consumer_files: usize,
    pub font_profile: String,
    pub font_sha256: String,
    pub font_version: String,
    pub font_source: String,
    pub font_upstream_revision: String,
    pub writes: Vec<PayloadWriteReport>,
}

#[derive(Debug)]
pub(crate) struct PatchedInstallerPayload {
    pub files: BTreeMap<String, Vec<u8>>,
    pub report: HangulVisibilityPatchReport,
}

pub(crate) fn build_hangul_visibility_payload(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> Result<PatchedInstallerPayload> {
    let gaiji_com = installer_payload
        .get(GAIJI_FILE)
        .context("verified installer payload is missing GAIJI.COM")?;
    let mad_com = installer_payload
        .get(DIALOGUE_FILE)
        .context("verified installer payload is missing MAD.COM")?;
    let gaiji_catalog = parse_gaiji_program(gaiji_com)?;
    let selected_glyph = gaiji_catalog
        .glyphs
        .get(DEMONSTRATION_GAIJI_SLOT)
        .context("configured Hangul GAIJI slot lies outside the installed table")?;
    ensure!(
        selected_glyph.file_offset == DEMONSTRATION_GAIJI_RECORD_OFFSET
            && selected_glyph.byte_size == GAIJI_RECORD_SIZE
            && selected_glyph.character_code == DEMONSTRATION_CHARACTER_CODE
            && selected_glyph.shift_jis_code == DEMONSTRATION_SHIFT_JIS_CODE,
        "configured Hangul GAIJI slot no longer matches the supported source"
    );
    let selected_record_end = selected_glyph.file_offset + selected_glyph.byte_size;
    let selected_record = GaijiRecord::parse(
        gaiji_com
            .get(selected_glyph.file_offset..selected_record_end)
            .context("configured GAIJI record lies outside GAIJI.COM")?,
    )?;
    selected_record.require_target_format()?;
    ensure!(
        selected_record.to_bytes() == EXPECTED_GAIJI_RECORD,
        "configured GAIJI record differs from the supported source preimage"
    );
    ensure!(
        mad_com.get(DEMONSTRATION_INTERFACE_OFFSET..DEMONSTRATION_INTERFACE_OFFSET + 2)
            == Some(EXPECTED_INTERFACE_CHARACTER.as_slice()),
        "configured interface character differs from the supported source preimage"
    );
    ensure!(
        mad_com.get(DEMONSTRATION_DIALOGUE_OFFSET..DEMONSTRATION_DIALOGUE_OFFSET + 2)
            == Some(EXPECTED_DIALOGUE_CHARACTER.as_slice()),
        "configured dialogue character differs from the supported source preimage"
    );

    let usage = require_unreferenced_gaiji_code(
        DEMONSTRATION_SHIFT_JIS_CODE,
        installer_payload,
        runtime_files,
        boot_files,
    )?;
    let bitmap = rasterize_hangul_syllable(DEMONSTRATION_CHARACTER)?;
    let replacement_record = GaijiRecord::from_bitmap(bitmap).to_bytes();
    let file_plans = vec![
        gaiji_file_plan(replacement_record),
        mad_file_plan(DEMONSTRATION_SHIFT_JIS_CODE.to_be_bytes()),
    ];
    let applied = apply_payload_write_plans(installer_payload, file_plans)?;
    let font = font_provenance()?;

    Ok(PatchedInstallerPayload {
        files: applied.files,
        report: HangulVisibilityPatchReport {
            character: DEMONSTRATION_CHARACTER.to_string(),
            gaiji_slot: DEMONSTRATION_GAIJI_SLOT,
            character_code: format!("0x{DEMONSTRATION_CHARACTER_CODE:04X}"),
            shift_jis_code: format!("0x{DEMONSTRATION_SHIFT_JIS_CODE:04X}"),
            interface_offset: DEMONSTRATION_INTERFACE_OFFSET,
            dialogue_file: DIALOGUE_FILE.to_owned(),
            dialogue_offset: DEMONSTRATION_DIALOGUE_OFFSET,
            audited_consumer_files: usage.audited_file_count,
            font_profile: font.profile_id,
            font_sha256: font.font_sha256,
            font_version: font.font_version,
            font_source: font.source,
            font_upstream_revision: font.upstream_revision,
            writes: applied.report,
        },
    })
}

fn gaiji_file_plan(replacement: [u8; GAIJI_RECORD_SIZE]) -> PayloadFileWritePlan {
    PayloadFileWritePlan {
        file_name: GAIJI_FILE,
        plan: WritePlan::new()
            .region(ImageRegion {
                id: "hangul-gaiji-record".to_owned(),
                range: DEMONSTRATION_GAIJI_RECORD_OFFSET
                    ..DEMONSTRATION_GAIJI_RECORD_OFFSET + GAIJI_RECORD_SIZE,
                kind: RegionKind::Data,
                reason: "one target-format BIOS GAIJI record".to_owned(),
            })
            .write(ExpectedWrite {
                id: "hangul-ga-glyph".to_owned(),
                owner: "hangul-font-rasterizer".to_owned(),
                purpose: "install the visibility-demonstration Hangul glyph".to_owned(),
                offset: DEMONSTRATION_GAIJI_RECORD_OFFSET,
                expected_original: EXPECTED_GAIJI_RECORD.to_vec(),
                replacement: replacement.to_vec(),
                intent: WriteIntent::Data,
            }),
    }
}

fn mad_file_plan(replacement: [u8; 2]) -> PayloadFileWritePlan {
    PayloadFileWritePlan {
        file_name: DIALOGUE_FILE,
        plan: WritePlan::new()
            .region(ImageRegion {
                id: "interface-leading-character".to_owned(),
                range: DEMONSTRATION_INTERFACE_OFFSET..DEMONSTRATION_INTERFACE_OFFSET + 2,
                kind: RegionKind::Data,
                reason: "first GAIJI code in the new-game menu's continue label".to_owned(),
            })
            .region(ImageRegion {
                id: "dialogue-leading-character".to_owned(),
                range: DEMONSTRATION_DIALOGUE_OFFSET..DEMONSTRATION_DIALOGUE_OFFSET + 2,
                kind: RegionKind::Data,
                reason: "first code unit in the first catalogued dialogue".to_owned(),
            })
            .write(ExpectedWrite {
                id: "interface-leading-hangul-code".to_owned(),
                owner: "interface-text-encoder".to_owned(),
                purpose: "expose the installed Hangul glyph on the first reachable menu".to_owned(),
                offset: DEMONSTRATION_INTERFACE_OFFSET,
                expected_original: EXPECTED_INTERFACE_CHARACTER.to_vec(),
                replacement: replacement.to_vec(),
                intent: WriteIntent::Data,
            })
            .write(ExpectedWrite {
                id: "dialogue-leading-hangul-code".to_owned(),
                owner: "dialogue-encoder".to_owned(),
                purpose:
                    "route one reachable dialogue character through the installed Hangul glyph"
                        .to_owned(),
                offset: DEMONSTRATION_DIALOGUE_OFFSET,
                expected_original: EXPECTED_DIALOGUE_CHARACTER.to_vec(),
                replacement: replacement.to_vec(),
                intent: WriteIntent::Data,
            }),
    }
}

#[cfg(test)]
#[path = "hangul_visibility_tests.rs"]
mod hangul_visibility_tests;
