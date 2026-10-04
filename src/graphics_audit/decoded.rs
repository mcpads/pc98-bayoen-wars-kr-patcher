use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use crate::localization_assets::decode_all_streams;

pub(super) fn decode_single_asset(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<Vec<u8>> {
    let packed = installer_payload
        .get(name)
        .with_context(|| format!("verified installer payload is missing {name}"))?;
    let streams = decode_all_streams(packed)?;
    ensure!(
        streams.len() == 1,
        "{name} does not contain exactly one Compile LZ stream"
    );
    Ok(streams.into_iter().next().unwrap().output)
}
