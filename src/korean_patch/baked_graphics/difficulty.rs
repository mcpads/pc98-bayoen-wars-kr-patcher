use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Result, ensure};

use super::difficulty_style::DifficultyLabelCanvas;
use crate::asset_bindings::BakedTextUnit;
use crate::translation_drafts::TranslationDraftEntry;

const TILE_WIDTH: usize = 16;
const TILE_HEIGHT: usize = 16;
const TILE_SIZE: usize = TILE_WIDTH / 8 * TILE_HEIGHT * 4;
const TILE_COUNT: usize = 57;
const TILE_COLUMNS: usize = 40;
const TILE_ROWS: usize = 25;
const TILE_ATLAS_SIZE: usize = TILE_SIZE * TILE_COUNT;
const PANEL_COLUMNS: usize = 12;
const PANEL_ROWS: usize = 2;

pub(super) fn compile_difficulty(
    source: &[u8],
    units: &[(&BakedTextUnit, &TranslationDraftEntry)],
) -> Result<Vec<u8>> {
    ensure!(
        source.len() == TILE_ATLAS_SIZE + TILE_COLUMNS * TILE_ROWS,
        "SEL1.DAT decoded size changed"
    );
    ensure!(
        units.len() == 3,
        "difficulty producer needs exactly three units"
    );
    let map = &source[TILE_ATLAS_SIZE..];
    let global_locations = tile_locations(map)?;
    let mut output = source.to_vec();

    for (binding, draft) in units {
        ensure!(
            binding.source_asset == "SEL1.DAT"
                && binding.producer_group_id == "selection-difficulty-labels"
                && binding.screen_regions.len() == 1,
            "{} has an unexpected difficulty producer binding",
            binding.id
        );
        let region = &binding.screen_regions[0];
        ensure!(
            region.x == 0
                && region.width == PANEL_COLUMNS * TILE_WIDTH
                && region.height == PANEL_ROWS * TILE_HEIGHT
                && region.y.is_multiple_of(TILE_HEIGHT),
            "{} difficulty screen region changed",
            binding.id
        );
        let map_row = region.y / TILE_HEIGHT;
        let panel_locations = (0..PANEL_ROWS)
            .flat_map(|row| (0..PANEL_COLUMNS).map(move |column| (map_row + row, column)))
            .collect::<BTreeSet<_>>();
        let writable_tiles = panel_locations
            .iter()
            .filter_map(|location| {
                let index = usize::from(map[location.0 * TILE_COLUMNS + location.1]);
                (global_locations[&index]
                    .iter()
                    .all(|candidate| panel_locations.contains(candidate)))
                .then_some(index)
            })
            .collect::<BTreeSet<_>>();
        ensure!(
            !writable_tiles.is_empty(),
            "{} has no panel-owned tiles",
            binding.id
        );

        ensure!(
            draft.korean_text.len() == 1,
            "{} difficulty label must use one line",
            draft.id
        );
        let writable_columns = (0..PANEL_COLUMNS)
            .filter(|column| {
                (0..PANEL_ROWS).all(|row| {
                    let index = usize::from(map[(map_row + row) * TILE_COLUMNS + column]);
                    writable_tiles.contains(&index) && global_locations[&index].len() == 1
                })
            })
            .collect::<Vec<_>>();
        ensure!(
            !writable_columns.is_empty()
                && writable_columns
                    .windows(2)
                    .all(|columns| columns[0] + 1 == columns[1]),
            "{} difficulty label has no contiguous two-row tile span",
            draft.id
        );
        let canvas = DifficultyLabelCanvas::render(
            &draft.korean_text[0],
            writable_columns.len() * TILE_WIDTH,
        )?;
        for (local_column, map_column) in writable_columns.into_iter().enumerate() {
            for row in 0..PANEL_ROWS {
                let tile_index = usize::from(map[(map_row + row) * TILE_COLUMNS + map_column]);
                canvas.write_tile(
                    &mut output,
                    tile_index * TILE_SIZE,
                    local_column * TILE_WIDTH,
                    row * TILE_HEIGHT,
                )?;
            }
        }
    }
    Ok(output)
}

fn tile_locations(map: &[u8]) -> Result<BTreeMap<usize, Vec<(usize, usize)>>> {
    let mut locations = BTreeMap::<usize, Vec<(usize, usize)>>::new();
    for row in 0..TILE_ROWS {
        for column in 0..TILE_COLUMNS {
            let index = usize::from(map[row * TILE_COLUMNS + column]);
            ensure!(index < TILE_COUNT, "SEL1 tile map references tile {index}");
            locations.entry(index).or_default().push((row, column));
        }
    }
    Ok(locations)
}

#[cfg(test)]
#[path = "difficulty_tests.rs"]
mod difficulty_tests;
