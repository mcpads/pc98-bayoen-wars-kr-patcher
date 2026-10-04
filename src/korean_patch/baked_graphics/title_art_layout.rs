use anyhow::{Context, Result, ensure};

use crate::pc98_graphics::{DIGITAL_RGBI_COLOR_COUNT, nearest_16_color_palette_index};

pub(super) struct RgbFrame {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) pixels: Vec<u8>,
}

#[derive(Clone, Copy)]
pub(super) struct PixelRect {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl PixelRect {
    fn contains(self, x: usize, y: usize) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }
}

#[derive(Clone, Copy)]
pub(super) struct SamplingPlacement {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) width: usize,
    pub(super) height: usize,
}

#[derive(Clone, Copy)]
pub(super) struct TitleFrameLayout {
    pub(super) sampling_width: usize,
    pub(super) sampling_height: usize,
    pub(super) placement: SamplingPlacement,
    pub(super) output_width: usize,
    pub(super) output_height: usize,
    pub(super) background_palette_index: u8,
}

pub(super) fn place_and_quantize_title_frame(
    artwork: &RgbFrame,
    crop: PixelRect,
    preserved_component_regions: &[PixelRect],
    layout: TitleFrameLayout,
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> Result<Vec<u8>> {
    ensure!(
        artwork.pixels.len()
            == artwork
                .width
                .checked_mul(artwork.height)
                .and_then(|pixels| pixels.checked_mul(3))
                .context("title artwork RGB size overflow")?,
        "title artwork RGB buffer size differs from its dimensions"
    );
    ensure!(
        usize::from(layout.background_palette_index) < palette.len(),
        "title artwork background index exceeds its palette"
    );
    ensure!(
        layout.sampling_width > 0
            && layout.sampling_height > 0
            && layout.output_width.is_multiple_of(layout.sampling_width)
            && layout.output_height.is_multiple_of(layout.sampling_height),
        "title artwork sampling canvas must divide its output"
    );
    ensure!(
        crop.width > 0
            && crop.height > 0
            && crop
                .x
                .checked_add(crop.width)
                .is_some_and(|end| end <= artwork.width)
            && crop
                .y
                .checked_add(crop.height)
                .is_some_and(|end| end <= artwork.height),
        "title artwork crop exceeds its RGB frame"
    );
    ensure!(
        preserved_component_regions.iter().all(|region| {
            region.width > 0
                && region.height > 0
                && region
                    .x
                    .checked_add(region.width)
                    .is_some_and(|end| end <= artwork.width)
                && region
                    .y
                    .checked_add(region.height)
                    .is_some_and(|end| end <= artwork.height)
        }),
        "title artwork preserved component region exceeds its RGB frame"
    );
    ensure!(
        layout.placement.width > 0
            && layout.placement.height > 0
            && layout
                .placement
                .x
                .checked_add(layout.placement.width)
                .is_some_and(|end| end <= layout.sampling_width)
            && layout
                .placement
                .y
                .checked_add(layout.placement.height)
                .is_some_and(|end| end <= layout.sampling_height),
        "title artwork placement exceeds its sampling canvas"
    );

    let mut sampled =
        vec![layout.background_palette_index; layout.sampling_width * layout.sampling_height];
    for local_y in 0..layout.placement.height {
        let source_y = crop.y + local_y * crop.height / layout.placement.height;
        let target_y = layout.placement.y + local_y;
        for local_x in 0..layout.placement.width {
            let source_x = crop.x + local_x * crop.width / layout.placement.width;
            let target_x = layout.placement.x + local_x;
            if preserved_component_regions
                .iter()
                .any(|region| region.contains(source_x, source_y))
            {
                sampled[target_y * layout.sampling_width + target_x] =
                    layout.background_palette_index;
                continue;
            }
            let source_offset = (source_y * artwork.width + source_x) * 3;
            sampled[target_y * layout.sampling_width + target_x] = nearest_16_color_palette_index(
                [
                    artwork.pixels[source_offset],
                    artwork.pixels[source_offset + 1],
                    artwork.pixels[source_offset + 2],
                ],
                palette,
            );
        }
    }

    let mut output = Vec::with_capacity(layout.output_width * layout.output_height);
    for y in 0..layout.output_height {
        let sample_y = y * layout.sampling_height / layout.output_height;
        for x in 0..layout.output_width {
            let sample_x = x * layout.sampling_width / layout.output_width;
            output.push(sampled[sample_y * layout.sampling_width + sample_x]);
        }
    }
    Ok(output)
}

#[cfg(test)]
#[path = "title_art_layout_tests.rs"]
mod title_art_layout_tests;
