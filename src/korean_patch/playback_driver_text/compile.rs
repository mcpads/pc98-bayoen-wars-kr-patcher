use anyhow::{Context, Result, ensure};

use super::layout::place_records_in_source_storage;
use super::model::CompiledPlaybackDriver;
use super::plans::{UnpackedLayout, apply_unpacked_layout};
use super::records::compile_records;
use super::typed_sources::assemble_playback_driver_sources;
use crate::dos_program::{pack_self_expanding_com, unpack_self_expanding_com};
use crate::external_text::{ExternalProgramTextCatalog, PackedSoundDriverRuntimeCatalog};
use crate::game_data::{GaijiCatalog, GaijiReadinessCatalog};
use crate::korean_patch::shared_text::{
    InstallerInterruptPolicy, assemble_embedded_gaiji_installer, compile_gaiji_bank,
};
use crate::translation_drafts::TranslationDraftSegment;

pub(super) struct PlaybackDriverCompileInput<'a> {
    pub file_name: &'static str,
    pub packed_source: &'a [u8],
    pub gaiji_com: &'a [u8],
    pub gaiji: &'a GaijiCatalog,
    pub readiness: &'a GaijiReadinessCatalog,
    pub source: &'a ExternalProgramTextCatalog,
    pub runtime: PackedSoundDriverRuntimeCatalog,
    pub draft: &'a TranslationDraftSegment,
}

pub(super) fn compile_playback_driver(
    input: PlaybackDriverCompileInput<'_>,
) -> Result<CompiledPlaybackDriver> {
    let PlaybackDriverCompileInput {
        file_name,
        packed_source,
        gaiji_com,
        gaiji,
        readiness,
        source,
        runtime,
        draft,
    } = input;
    ensure!(
        source.file_name == file_name && runtime.file_name == file_name,
        "playback-driver source identities disagree"
    );
    let source_image = unpack_self_expanding_com(packed_source)?;
    let source_unpacked = source_image.unpacked;
    let draft_lines = draft
        .entries
        .iter()
        .flat_map(|entry| entry.korean_text.iter().map(String::as_str))
        .collect::<Vec<_>>();
    let bank = compile_gaiji_bank(gaiji_com, gaiji, readiness, &draft_lines)?;
    let mut entries = compile_records(&source_unpacked, source, draft, &bank)?;
    let (text_storage_writes, text_storage_capacity, text_storage_used) =
        place_records_in_source_storage(&source_unpacked, source, &mut entries)?;

    let installer_offset = runtime.resident_end_file_offset;
    ensure!(
        source_unpacked.len() < installer_offset,
        "{file_name} has no verified gap before the first non-resident file byte"
    );
    let (installer, glyph_records) = assemble_embedded_gaiji_installer(
        gaiji,
        &bank,
        installer_offset,
        runtime.transient_entry_com_address,
        InstallerInterruptPolicy::EnableAndRestore,
        &format!("{} playback driver", file_name.to_ascii_lowercase()),
    )?;
    let glyph_records_offset = installer_offset
        .checked_add(installer.bytes().len())
        .context("playback-driver glyph record offset overflow")?;
    let typed_sources = assemble_playback_driver_sources(
        &source_unpacked,
        &runtime,
        installer_offset,
        installer.clone(),
        &entries,
    )?;
    let (output_unpacked, unpacked_writes) = apply_unpacked_layout(
        &source_unpacked,
        UnpackedLayout {
            file_name,
            runtime: &runtime,
            typed_sources: &typed_sources,
            text_storage_writes: &text_storage_writes,
            entries: &entries,
            installer_offset,
            installer: &installer,
            glyph_records_offset,
            glyph_records: &glyph_records,
        },
    )?;
    let output_packed = pack_self_expanding_com(&output_unpacked)?;
    ensure!(
        unpack_self_expanding_com(&output_packed)?.unpacked == output_unpacked,
        "{file_name} deterministic repack failed its complete unpacked round trip"
    );

    Ok(CompiledPlaybackDriver {
        file_name,
        bank,
        runtime,
        source_packed_size: packed_source.len(),
        source_unpacked,
        output_unpacked,
        output_packed,
        installer_offset,
        installer,
        glyph_records_offset,
        glyph_records,
        text_storage_writes,
        text_storage_capacity,
        text_storage_used,
        entries,
        typed_sources,
        unpacked_writes,
    })
}
