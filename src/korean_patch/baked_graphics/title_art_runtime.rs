use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use expected_write::{
    ExpectedWrite, ImageRegion, MachineCodeProvenance, RegionKind, WriteIntent, WritePlan,
};
use v30::{AssembledProgram, Instruction, Operand, Register16, decode_bytes};

use super::model::TitleArtworkPatchReport;
use crate::korean_patch::payload_writes::PayloadFileWritePlan;
use crate::korean_patch::typed_v30::assemble_relocated_mov_immediate;
use crate::title_runtime::{
    SOURCE_TITLE_ANIMATION_FRAME_SOURCES, SOURCE_TITLE_PALETTE_RGB4,
    TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET, TITLE_PALETTE_TABLE_FILE_OFFSET, TitlePaletteRgb4,
    disabled_title_animation_frame_sources, encode_title_animation_frame_table,
    encode_title_palette_table, read_title_palette_rgb4, title_animation_is_disabled,
};

const MAD_COM: &str = "MAD.COM";
const PRODUCER_ID: &str = "title-art-runtime-configuration";
const TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET: usize = 0xac21;
const TITLE_ANIMATION_END_FALLBACK_SOURCE_ID: &str = "title-animation-end-fallback";
const SOURCE_TITLE_ANIMATION_END_FALLBACK: u16 = 0x0c00;

pub(super) struct PlannedTitleArtworkRuntime {
    pub(super) plan: PayloadFileWritePlan,
    pub(super) typed_sources: BTreeMap<String, AssembledProgram>,
}

pub(super) fn title_art_runtime_plan(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    palette: &TitlePaletteRgb4,
) -> Result<PlannedTitleArtworkRuntime> {
    let baseline = installer_payload
        .get(MAD_COM)
        .context("verified installer payload is missing MAD.COM")?;
    let expected_animation =
        encode_title_animation_frame_table(&SOURCE_TITLE_ANIMATION_FRAME_SOURCES);
    let replacement_animation =
        encode_title_animation_frame_table(&disabled_title_animation_frame_sources());
    let expected_palette = encode_title_palette_table(&SOURCE_TITLE_PALETTE_RGB4)?;
    let replacement_palette = encode_title_palette_table(palette)?;
    require_source_table(
        baseline,
        TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET,
        &expected_animation,
        "title animation frame",
    )?;
    require_source_table(
        baseline,
        TITLE_PALETTE_TABLE_FILE_OFFSET,
        &expected_palette,
        "title palette",
    )?;
    require_source_animation_end_fallback(baseline)?;

    let fallback_source = assemble_relocated_mov_immediate(
        baseline,
        TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET,
        crate::title_runtime::TITLE_ANIMATION_SKIP_SOURCE,
        &[Register16::AX],
        "title animation end fallback",
    )?;
    let fallback_replacement = fallback_source.bytes().to_vec();
    let fallback_range = TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET
        ..TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET + fallback_replacement.len();
    let fallback_expected = baseline
        .get(fallback_range.clone())
        .context("title animation end fallback lies outside MAD.COM")?
        .to_vec();

    let animation_range = TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET
        ..TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET + expected_animation.len();
    let mut plan = WritePlan::new()
        .region(ImageRegion {
            id: "title-animation-frame-table".to_owned(),
            range: animation_range,
            kind: RegionKind::Metadata,
            reason: "TITLE2 frame sources selected by the title consumer".to_owned(),
        })
        .write(ExpectedWrite {
            id: "disable-title2-animation-frames".to_owned(),
            owner: PRODUCER_ID.to_owned(),
            purpose: "prevent source-only TITLE2 artwork from overwriting replaced base artwork"
                .to_owned(),
            offset: TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET,
            expected_original: expected_animation,
            replacement: replacement_animation,
            intent: WriteIntent::Metadata,
        })
        .region(ImageRegion {
            id: "title-animation-end-fallback".to_owned(),
            range: fallback_range,
            kind: RegionKind::MachineCode,
            reason: "one complete typed V30 MOV AX, imm16 instruction".to_owned(),
        })
        .write(ExpectedWrite {
            id: "disable-title2-animation-end-fallback".to_owned(),
            owner: PRODUCER_ID.to_owned(),
            purpose: "prevent the animation loop end marker from restoring TITLE2 frame zero"
                .to_owned(),
            offset: TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET,
            expected_original: fallback_expected,
            replacement: fallback_replacement,
            intent: WriteIntent::MachineCode(MachineCodeProvenance {
                assembly_source_id: TITLE_ANIMATION_END_FALLBACK_SOURCE_ID.to_owned(),
                isa_profile_id: v30::PROFILE_ID.to_owned(),
            }),
        });
    if palette != &SOURCE_TITLE_PALETTE_RGB4 {
        let palette_range = TITLE_PALETTE_TABLE_FILE_OFFSET
            ..TITLE_PALETTE_TABLE_FILE_OFFSET + expected_palette.len();
        plan = plan
            .region(ImageRegion {
                id: "title-runtime-palette-table".to_owned(),
                range: palette_range,
                kind: RegionKind::Metadata,
                reason: "PC-98 RGB4 colors programmed only for the title scene".to_owned(),
            })
            .write(ExpectedWrite {
                id: "replace-title-runtime-palette".to_owned(),
                owner: PRODUCER_ID.to_owned(),
                purpose: "program the RGB4 palette used to quantize the replacement title artwork"
                    .to_owned(),
                offset: TITLE_PALETTE_TABLE_FILE_OFFSET,
                expected_original: expected_palette,
                replacement: replacement_palette,
                intent: WriteIntent::Metadata,
            });
    }
    Ok(PlannedTitleArtworkRuntime {
        plan: PayloadFileWritePlan {
            file_name: MAD_COM,
            plan,
        },
        typed_sources: BTreeMap::from([(
            TITLE_ANIMATION_END_FALLBACK_SOURCE_ID.to_owned(),
            fallback_source,
        )]),
    })
}

pub(super) fn verify_title_art_runtime(
    files: &BTreeMap<String, Vec<u8>>,
    report: &TitleArtworkPatchReport,
) -> Result<()> {
    let mad_com = files
        .get(MAD_COM)
        .context("patched payload is missing MAD.COM")?;
    ensure!(
        read_title_palette_rgb4(mad_com)? == report.runtime_palette_rgb4,
        "title runtime palette failed final payload readback"
    );
    ensure!(
        title_animation_is_disabled(mad_com)? && !report.title2_animation_enabled,
        "TITLE2 animation remains enabled after title artwork replacement"
    );
    ensure!(
        title_animation_end_fallback_is_disabled(mad_com)?
            && report.title2_animation_end_fallback_disabled,
        "TITLE2 animation end fallback remains enabled after title artwork replacement"
    );
    Ok(())
}

fn require_source_animation_end_fallback(mad_com: &[u8]) -> Result<()> {
    let decoded = decode_bytes(
        mad_com
            .get(TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET..)
            .context("title animation end fallback lies outside MAD.COM")?,
    )?;
    ensure!(
        decoded.prefixes.is_empty()
            && decoded.byte_len == 3
            && decoded.instruction
                == Instruction::Mov {
                    dest: Operand::Reg16(Register16::AX),
                    src: Operand::Imm16(SOURCE_TITLE_ANIMATION_END_FALLBACK),
                },
        "MAD.COM source title animation end fallback changed"
    );
    Ok(())
}

fn title_animation_end_fallback_is_disabled(mad_com: &[u8]) -> Result<bool> {
    let decoded = decode_bytes(
        mad_com
            .get(TITLE_ANIMATION_END_FALLBACK_FILE_OFFSET..)
            .context("title animation end fallback lies outside MAD.COM")?,
    )?;
    Ok(decoded.prefixes.is_empty()
        && decoded.byte_len == 3
        && decoded.instruction
            == Instruction::Mov {
                dest: Operand::Reg16(Register16::AX),
                src: Operand::Imm16(crate::title_runtime::TITLE_ANIMATION_SKIP_SOURCE),
            })
}

fn require_source_table(mad_com: &[u8], offset: usize, expected: &[u8], role: &str) -> Result<()> {
    ensure!(
        mad_com.get(offset..offset + expected.len()) == Some(expected),
        "MAD.COM source {role} table changed"
    );
    Ok(())
}

#[cfg(test)]
#[path = "title_art_runtime_tests.rs"]
mod title_art_runtime_tests;
