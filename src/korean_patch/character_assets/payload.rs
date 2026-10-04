use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::model::{ArleCharacterFileReport, PreparedArleAssetFile};
use crate::asset_bindings::catalog_character_sprites;
use crate::localization_assets::decode_all_streams;
use crate::source_disk::sha256_hex;

pub(super) fn verify_replacement_readback(
    mad_com: &[u8],
    files: &BTreeMap<String, Vec<u8>>,
    replacements: &[PreparedArleAssetFile],
) -> Result<()> {
    let catalog = catalog_character_sprites(mad_com, files)?;
    ensure!(
        catalog.slot_count == 18 && catalog.asset_count == 21,
        "character catalog changed after Arle replacement"
    );
    for replacement in replacements {
        let decoded = decode_single_asset(files, replacement.file_name)?;
        ensure!(
            decoded == replacement.decoded,
            "Arle {} pixels did not survive recompression readback",
            replacement.file_name
        );
    }
    Ok(())
}

pub(super) fn file_report(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    replacement: &PreparedArleAssetFile,
) -> Result<ArleCharacterFileReport> {
    let streams = decode_all_streams(&replacement.packed)?;
    ensure!(
        streams.len() == 1,
        "Arle replacement must contain one Compile LZ stream"
    );
    Ok(ArleCharacterFileReport {
        file_name: replacement.file_name.to_owned(),
        decoded_size: replacement.decoded.len(),
        decoded_sha256: sha256_hex(&replacement.decoded),
        original_packed_size: required(installer_payload, replacement.file_name)?.len(),
        replacement_packed_size: replacement.packed.len(),
        replacement_packed_sha256: sha256_hex(&replacement.packed),
        replacement_decode_command_count: streams[0].command_count,
    })
}

pub(super) fn decode_single_asset(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    file_name: &str,
) -> Result<Vec<u8>> {
    let streams = decode_all_streams(required(installer_payload, file_name)?)?;
    ensure!(
        streams.len() == 1,
        "{file_name} does not contain one Compile LZ stream"
    );
    Ok(streams.into_iter().next().unwrap().output)
}

pub(super) fn required<'a>(
    installer_payload: &'a BTreeMap<String, Vec<u8>>,
    file_name: &str,
) -> Result<&'a Vec<u8>> {
    installer_payload
        .get(file_name)
        .with_context(|| format!("verified installer payload is missing {file_name}"))
}
