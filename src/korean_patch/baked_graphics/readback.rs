use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, ensure};

use super::model::AssetPatch;
use crate::asset_bindings::catalog_baked_text;
use crate::localization_assets::decode_all_streams;

pub(super) fn verify_baked_payload(
    files: &BTreeMap<String, Vec<u8>>,
    patches: &[AssetPatch],
) -> Result<()> {
    for patch in patches {
        let streams = decode_all_streams(&files[patch.file_name])?;
        ensure!(
            streams.len() == 1 && streams[0].output == patch.decoded,
            "{} baked graphics failed decoded readback",
            patch.file_name
        );
    }
    let mad_com = files
        .get("MAD.COM")
        .context("patched payload is missing MAD.COM")?;
    let catalog = catalog_baked_text(mad_com, files)?;
    let expected_ids = patches
        .iter()
        .flat_map(|patch| patch.unit_ids.iter().map(String::as_str))
        .collect::<BTreeSet<_>>();
    let actual_ids = catalog
        .units
        .iter()
        .map(|unit| unit.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure!(
        catalog.translation_unit_count == 16
            && catalog.consumer_blocks.len() == 5
            && expected_ids == actual_ids,
        "baked graphics consumer population changed after readback"
    );
    Ok(())
}
