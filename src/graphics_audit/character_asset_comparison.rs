use std::collections::BTreeMap;

use anyhow::{Context, Result};

use super::Preview;
use super::character_sprites::render_named_character_sprite_preview;
use super::contact_sheet::compose_contact_sheet;

const COMPARISON_WIDTH: usize = 1280;
const COMPARISON_GAP: usize = 16;

pub(super) fn render_arle_character_comparisons(
    source: &BTreeMap<String, Vec<u8>>,
    replacement: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let source_mad = source
        .get("MAD.COM")
        .context("source payload is missing MAD.COM")?;
    let replacement_mad = replacement
        .get("MAD.COM")
        .context("replacement payload is missing MAD.COM")?;
    ["C07", "C08"]
        .into_iter()
        .map(|asset_name| {
            let source_preview =
                render_named_character_sprite_preview(source_mad, source, asset_name)?;
            let replacement_preview = render_named_character_sprite_preview(
                replacement_mad,
                replacement,
                asset_name,
            )?;
            Ok(Preview {
                source_asset: asset_name.to_owned(),
                output_file: format!(
                    "arle-{}-source-vs-replacement.png",
                    asset_name.to_ascii_lowercase()
                ),
                evidence: format!(
                    "left: immutable source {asset_name}; right: project-authored Arle frame set after Bayoen layout, MAD.COM file 0xD777 runtime RGB4 quantization, B/R/G/I packing, Compile LZ recompression, and readback"
                ),
                image: compose_contact_sheet(
                    vec![source_preview.image, replacement_preview.image],
                    COMPARISON_WIDTH,
                    COMPARISON_GAP,
                )?,
            })
        })
        .collect()
}
