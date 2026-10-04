use std::collections::BTreeMap;

use anyhow::Result;

use super::Preview;
use super::decoded::decode_single_asset;
use super::tile_sheet::render_compact_brgi_tiles;

pub(super) fn render_diagnostic_previews(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let date = decode_single_asset(installer_payload, "DATE_.DAT")?;
    Ok(vec![Preview {
        source_asset: "DATE_.DAT".to_owned(),
        output_file: "date-8x16-records.png".to_owned(),
        evidence: "DATE_.DAT partitions exactly into seventeen independent 8x16 B/R/G/I records; a direct MAD.COM display consumer is not yet proven"
            .to_owned(),
        image: render_compact_brgi_tiles(&date, 1, 16, 17)?,
    }])
}
