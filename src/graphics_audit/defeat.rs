use std::collections::BTreeMap;

use anyhow::{Result, ensure};

use super::Preview;
use super::decoded::decode_single_asset;
use super::planar::PlanarScreen;

const TRANSFERS: [(usize, usize, usize, usize); 22] = [
    (0x0000, 0x1908, 4, 16),
    (0x0200, 0x1e06, 2, 32),
    (0x0400, 0x1e16, 1, 16),
    (0x0480, 0x1e20, 1, 16),
    (0x0500, 0x192e, 2, 16),
    (0x0600, 0x1e2c, 4, 16),
    (0x0800, 0x232e, 2, 16),
    (0x0900, 0x3c08, 4, 32),
    (0x0d00, 0x460a, 2, 16),
    (0x0e00, 0x3c1a, 1, 16),
    (0x0e80, 0x3c20, 1, 32),
    (0x0f80, 0x3722, 1, 16),
    (0x1000, 0x372c, 1, 16),
    (0x1080, 0x4134, 1, 16),
    (0x1100, 0x5f08, 1, 16),
    (0x1180, 0x6406, 4, 32),
    (0x1580, 0x6e06, 3, 16),
    (0x1700, 0x6918, 3, 16),
    (0x1880, 0x6420, 1, 32),
    (0x1980, 0x692c, 4, 16),
    (0x1b80, 0x6e2e, 3, 16),
    (0x1d00, 0x0000, 0, 0),
];

pub(super) fn render_defeat_preview(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Preview> {
    let source = decode_single_asset(installer_payload, "DEFEAT.DAT")?;
    ensure!(
        source.len() == 0x1d00,
        "DEFEAT.DAT decoded size does not match its transfer tables"
    );
    let mut screen = PlanarScreen::new();
    for (source_offset, destination_offset, width_words, height) in
        TRANSFERS[..TRANSFERS.len() - 1].iter().copied()
    {
        screen.copy_compact_brgi(
            &source,
            source_offset,
            destination_offset,
            width_words * 2,
            height,
        )?;
    }
    Ok(Preview {
        source_asset: "DEFEAT.DAT".to_owned(),
        output_file: "defeat-consumer-layout.png".to_owned(),
        evidence: "MAD.COM files 0xA67B and 0xA69D bind nine state groups and 21 B/R/G/I transfers that consume all decoded bytes"
            .to_owned(),
        image: screen.into_rgb(),
    })
}

#[cfg(test)]
#[path = "defeat_tests.rs"]
mod defeat_tests;
