use anyhow::{Context, Result, ensure};

use super::canvas::TextCanvas;
use crate::asset_bindings::BakedTextUnit;
use crate::korean_patch::font_catalog::FontTarget;
use crate::title_screen::{TITLE_CONTENT_TRANSFERS, TITLE_DECODED_SIZE};
use crate::translation_drafts::TranslationDraftEntry;

pub(super) fn compile_title(
    source: &[u8],
    units: &[(&BakedTextUnit, &TranslationDraftEntry)],
) -> Result<Vec<u8>> {
    ensure!(
        source.len() == TITLE_DECODED_SIZE,
        "TITLE.DAT decoded size changed"
    );
    ensure!(units.len() == 2, "title producer needs exactly two units");
    let mut output = source.to_vec();
    for (binding, draft) in units {
        ensure!(
            binding.source_asset == "TITLE.DAT"
                && binding.producer_group_id == "title-logo"
                && binding.screen_regions.len() == 1,
            "{} has an unexpected title producer binding",
            binding.id
        );
        let region = &binding.screen_regions[0];
        let target = match binding.id.as_str() {
            "title-logo-primary" => FontTarget::TitlePrimary16,
            "title-logo-subtitle" => FontTarget::BakedDisplay16,
            _ => anyhow::bail!("{} has no title font target", binding.id),
        };
        let canvas = TextCanvas::render(
            target,
            &draft.korean_text,
            region.width,
            region.height,
            2,
            false,
        )?;
        write_title_region(&mut output, region.x, region.y, &canvas)?;
    }
    Ok(output)
}

fn write_title_region(
    decoded: &mut [u8],
    screen_x: usize,
    screen_y: usize,
    canvas: &TextCanvas,
) -> Result<()> {
    for local_y in 0..canvas.height {
        for local_x in 0..canvas.width {
            let x = screen_x + local_x;
            let y = screen_y + local_y;
            let transfer = TITLE_CONTENT_TRANSFERS
                .iter()
                .copied()
                .find(|transfer| transfer.contains(x, y))
                .with_context(|| format!("title screen pixel ({x},{y}) has no source transfer"))?;
            let local_transfer_x = x - transfer.x();
            let local_transfer_y = y - transfer.y();
            let plane_size = transfer.width_bytes * transfer.height;
            let byte = local_transfer_y * transfer.width_bytes + local_transfer_x / 8;
            let mask = 0x80 >> (local_transfer_x % 8);
            for plane in 0..4 {
                let offset = transfer.source_offset + plane * plane_size + byte;
                let target = decoded
                    .get_mut(offset)
                    .context("title source transfer lies outside TITLE.DAT")?;
                *target &= !mask;
                if canvas.pixel(local_x, local_y) {
                    *target |= mask;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "title_tests.rs"]
mod title_tests;
