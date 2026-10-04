use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};

use super::Preview;
use super::pc98_text::{CELL_SIZE, draw_bitmap};
use super::planar::RgbImage;
use crate::game_data::{GaijiTextCell, catalog_game_data};
use crate::korean_patch::{FixedCellRasterizer, GLYPH_BYTES};

const COLUMN_COUNT: usize = 7;
const ROW_COUNT: usize = 6;

pub(super) fn render_fixed_gaiji_text_previews(
    installer_payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<Preview>> {
    let mad_com = installer_payload
        .get("MAD.COM")
        .context("fixed-text audit payload is missing MAD.COM")?;
    let gaiji_com = installer_payload
        .get("GAIJI.COM")
        .context("fixed-text audit payload is missing GAIJI.COM")?;
    let menu_com = installer_payload
        .get("MENU.COM")
        .context("fixed-text audit payload is missing MENU.COM")?;
    let catalog = catalog_game_data(mad_com, gaiji_com, menu_com)?;
    let rasterizer = FixedCellRasterizer::load()?;

    catalog
        .mad_com
        .fixed_gaiji_text
        .slots
        .iter()
        .map(|slot| {
            ensure!(
                slot.lines.len() == ROW_COUNT
                    && slot.lines.iter().all(|line| line.len() == COLUMN_COUNT),
                "{} fixed-text layout changed before rendering",
                slot.id
            );
            let mut image = RgbImage {
                width: COLUMN_COUNT * CELL_SIZE,
                height: ROW_COUNT * CELL_SIZE,
                pixels: vec![0; COLUMN_COUNT * CELL_SIZE * ROW_COUNT * CELL_SIZE * 3],
            };
            for (row, line) in slot.lines.iter().enumerate() {
                for (column, cell) in line.iter().enumerate() {
                    let bitmap = match cell {
                        GaijiTextCell::Space => continue,
                        GaijiTextCell::Standard { text } => {
                            let mut characters = text.chars();
                            let character = characters
                                .next()
                                .context("fixed-text standard cell is empty")?;
                            ensure!(
                                characters.next().is_none(),
                                "fixed-text standard cell has multiple characters"
                            );
                            rasterizer.rasterize_visible_character(character)?
                        }
                        GaijiTextCell::Glyph { glyph_index, .. } => {
                            let glyph = &catalog.gaiji.glyphs[*glyph_index];
                            let record = gaiji_com
                                .get(glyph.file_offset..glyph.file_offset + glyph.byte_size)
                                .context("fixed-text GAIJI record lies outside GAIJI.COM")?;
                            ensure!(
                                record.len() == GLYPH_BYTES + 2 && record[..2] == [0, 0],
                                "fixed-text GAIJI record format changed"
                            );
                            let mut bitmap = [0_u8; GLYPH_BYTES];
                            bitmap.copy_from_slice(&record[2..]);
                            bitmap
                        }
                    };
                    draw_bitmap(&mut image, column * CELL_SIZE, row * CELL_SIZE, &bitmap);
                }
            }
            Ok(Preview {
                source_asset: "MAD.COM + GAIJI.COM".to_owned(),
                output_file: format!("{}.png", slot.id),
                evidence: format!(
                    "MAD.COM fixed-text pointer at {:#06X} binds one 6x7 cell slot to installed GAIJI records",
                    slot.pointer_instruction_offset
                ),
                image,
            })
        })
        .collect()
}
