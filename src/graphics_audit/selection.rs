use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::decoded::decode_single_asset;
use super::planar::PlanarScreen;

const TILE_WIDTH_BYTES: usize = 2;
const TILE_HEIGHT: usize = 16;
const TILE_PLANE_SIZE: usize = TILE_WIDTH_BYTES * TILE_HEIGHT;
const TILE_SIZE: usize = TILE_PLANE_SIZE * 4;
const TILE_COLUMNS: usize = 40;
const TILE_ROWS: usize = 25;
const TILE_ATLAS_SIZE: usize = 0x1c80;
const TILE_MAP_SIZE: usize = TILE_COLUMNS * TILE_ROWS;

const OVERLAY_WIDTH_BYTES: usize = 16;
const OVERLAY_HEIGHT: usize = 96;
const OVERLAY_SIZE: usize = OVERLAY_WIDTH_BYTES * OVERLAY_HEIGHT * 4;
const OVERLAY_TRANSFERS: [(usize, usize); 10] = [
    (0x0000, 0x0a04),
    (0x1800, 0x0a16),
    (0x3000, 0x0a28),
    (0x4800, 0x3204),
    (0x6000, 0x3216),
    (0x7800, 0x3228),
    (0x9000, 0x5a04),
    (0xa800, 0x5a16),
    (0xc000, 0x5a28),
    (0xd800, 0x0a3e),
];

const SEL3_SPRITE_HEIGHT: usize = 32;
const SEL3_SPRITE_TRANSFERS: [(usize, usize, usize, usize); 10] = [
    (0x0000, 0x2304, 7, 0x01c0),
    (0x08c0, 0x2316, 7, 0x01c0),
    (0x1180, 0x2328, 10, 0x0280),
    (0x1e00, 0x4b04, 5, 0x0140),
    (0x2440, 0x4b16, 7, 0x01c0),
    (0x2d00, 0x4b28, 6, 0x0180),
    (0x3480, 0x7304, 5, 0x0140),
    (0x3ac0, 0x7316, 6, 0x0180),
    (0x4240, 0x7328, 4, 0x0100),
    (0x4740, 0x733e, 4, 0x0100),
];
const SEL3_UNBOUND_START: usize = 0x4c40;
const SEL3_UNBOUND_END: usize = 0x5340;
const SEL3_PANEL_TRANSFERS: [(usize, usize, usize, usize); 3] = [
    (0x5340, 0x002e, 32, 48),
    (0x6b40, 0x002e, 32, 48),
    (0x8340, 0x5a3e, 16, 96),
];

pub(super) fn render_selection_previews(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let tiles = decode_single_asset(installer_payload, "SEL1.DAT")?;
    let overlays = decode_single_asset(installer_payload, "SEL2.DAT")?;
    let animations = decode_single_asset(installer_payload, "SEL3.DAT")?;
    let mut tile_screen = PlanarScreen::new();
    let mut overlay_screen = PlanarScreen::new();

    render_tile_map(&mut tile_screen, &tiles)?;
    render_overlays(&mut overlay_screen, &overlays)?;

    let mut previews = vec![
        Preview {
            source_asset: "SEL1.DAT".to_owned(),
            output_file: "selection-tile-map.png".to_owned(),
            evidence: "MAD.COM file 0xA1F5 renders the SEL1 40x25 tile map from its 57-tile atlas"
                .to_owned(),
            image: tile_screen.into_rgb(),
        },
        Preview {
            source_asset: "SEL2.DAT".to_owned(),
            output_file: "selection-portrait-overlays.png".to_owned(),
            evidence: "MAD.COM selection transfer table consumes ten SEL2 128x96 portrait overlays"
                .to_owned(),
            image: overlay_screen.into_rgb(),
        },
    ];
    previews.push(render_sel3_sprites(&animations)?);
    previews.extend(render_sel3_panels(&animations)?);
    Ok(previews)
}

fn render_tile_map(screen: &mut PlanarScreen, source: &[u8]) -> Result<()> {
    ensure!(
        source.len() == TILE_ATLAS_SIZE + TILE_MAP_SIZE,
        "SEL1.DAT decoded size does not match its consumer layout"
    );
    for row in 0..TILE_ROWS {
        for column in 0..TILE_COLUMNS {
            let map_offset = TILE_ATLAS_SIZE + row * TILE_COLUMNS + column;
            let tile_index = usize::from(source[map_offset]);
            let source_offset = tile_index
                .checked_mul(TILE_SIZE)
                .context("SEL1 tile source offset overflow")?;
            ensure!(
                source_offset + TILE_SIZE <= TILE_ATLAS_SIZE,
                "SEL1 tile map references tile {tile_index} outside its atlas"
            );
            let destination_offset = row * TILE_HEIGHT * 80 + column * TILE_WIDTH_BYTES;
            screen.copy_compact_brgi(
                source,
                source_offset,
                destination_offset,
                TILE_WIDTH_BYTES,
                TILE_HEIGHT,
            )?;
        }
    }
    Ok(())
}

fn render_overlays(screen: &mut PlanarScreen, source: &[u8]) -> Result<()> {
    ensure!(
        source.len() == OVERLAY_SIZE * OVERLAY_TRANSFERS.len(),
        "SEL2.DAT decoded size does not match its consumer layout"
    );
    for (source_offset, destination_offset) in OVERLAY_TRANSFERS {
        screen.copy_compact_brgi(
            source,
            source_offset,
            destination_offset,
            OVERLAY_WIDTH_BYTES,
            OVERLAY_HEIGHT,
        )?;
    }
    Ok(())
}

fn render_sel3_sprites(source: &[u8]) -> Result<Preview> {
    ensure!(
        source.len() == 0x9b40,
        "SEL3.DAT decoded size does not match its consumer layout"
    );
    let mut screen = PlanarScreen::new();
    for (source_offset, destination_offset, width_words, plane_stride) in SEL3_SPRITE_TRANSFERS {
        let width_bytes = width_words * 2;
        ensure!(
            plane_stride == width_bytes * SEL3_SPRITE_HEIGHT,
            "SEL3 sprite plane stride does not match its dimensions"
        );
        screen.copy_compact_brgi(
            source,
            source_offset + plane_stride,
            destination_offset,
            width_bytes,
            SEL3_SPRITE_HEIGHT,
        )?;
    }
    Ok(Preview {
        source_asset: "SEL3.DAT".to_owned(),
        output_file: "sel3-sprite-overlays.png".to_owned(),
        evidence: format!(
            "MAD.COM file 0xA32E table binds ten mask-plus-B/R/G/I overlays; decoded range {SEL3_UNBOUND_START:#06X}..{SEL3_UNBOUND_END:#06X} remains explicitly unbound"
        ),
        image: screen.into_rgb(),
    })
}

fn render_sel3_panels(source: &[u8]) -> Result<Vec<Preview>> {
    SEL3_PANEL_TRANSFERS
        .into_iter()
        .enumerate()
        .map(|(index, (source_offset, destination_offset, width_bytes, height))| {
            let mut screen = PlanarScreen::new();
            screen.copy_compact_brgi(
                source,
                source_offset,
                destination_offset,
                width_bytes,
                height,
            )?;
            Ok(Preview {
                source_asset: "SEL3.DAT".to_owned(),
                output_file: format!("sel3-panel-{}.png", index + 1),
                evidence: format!(
                    "MAD.COM selection consumer binds SEL3 source {source_offset:#06X} as a {}x{height} B/R/G/I panel",
                    width_bytes * 8
                ),
                image: screen.into_rgb(),
            })
        })
        .collect()
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod selection_tests;
