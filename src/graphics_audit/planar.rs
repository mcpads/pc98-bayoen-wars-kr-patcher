use anyhow::{Context, Result, ensure};

use crate::pc98_graphics::{DIGITAL_RGBI_COLOR_COUNT, digital_rgbi_palette};

pub(super) const SCREEN_WIDTH: usize = 640;
pub(super) const SCREEN_HEIGHT: usize = 400;
const SCREEN_ROW_BYTES: usize = SCREEN_WIDTH / 8;
const SCREEN_PLANE_BYTES: usize = SCREEN_ROW_BYTES * SCREEN_HEIGHT;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct RgbImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

pub(super) struct PlanarScreen {
    planes: [Vec<u8>; 4],
}

impl PlanarScreen {
    pub fn new() -> Self {
        Self {
            planes: std::array::from_fn(|_| vec![0; SCREEN_PLANE_BYTES]),
        }
    }

    pub fn copy_compact_brgi(
        &mut self,
        source: &[u8],
        source_offset: usize,
        destination_offset: usize,
        width_bytes: usize,
        height: usize,
    ) -> Result<()> {
        ensure!(
            width_bytes > 0 && width_bytes <= SCREEN_ROW_BYTES && height > 0,
            "invalid compact B/R/G/I dimensions"
        );
        let plane_size = width_bytes
            .checked_mul(height)
            .context("compact B/R/G/I plane size overflow")?;
        let source_end = source_offset
            .checked_add(plane_size * 4)
            .context("compact B/R/G/I source boundary overflow")?;
        ensure!(
            source_end <= source.len(),
            "compact B/R/G/I source ends at {source_end:#x}, past {:#x}",
            source.len()
        );

        for plane in 0..4 {
            for row in 0..height {
                let source_start = source_offset + plane * plane_size + row * width_bytes;
                let destination_start = destination_offset + row * SCREEN_ROW_BYTES;
                let destination_end = destination_start + width_bytes;
                ensure!(
                    destination_end <= SCREEN_PLANE_BYTES,
                    "compact B/R/G/I destination crosses graphics VRAM"
                );
                self.planes[plane][destination_start..destination_end]
                    .copy_from_slice(&source[source_start..source_start + width_bytes]);
            }
        }
        Ok(())
    }

    pub fn into_rgb(self) -> RgbImage {
        render_strided_brgi(
            &self.planes.concat(),
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            SCREEN_ROW_BYTES,
            SCREEN_PLANE_BYTES,
        )
        .expect("an internally sized planar screen always renders")
    }
}

pub(super) fn render_strided_brgi(
    source: &[u8],
    width: usize,
    height: usize,
    source_row_bytes: usize,
    plane_stride: usize,
) -> Result<RgbImage> {
    render_strided_brgi_with_palette(
        source,
        width,
        height,
        source_row_bytes,
        plane_stride,
        &digital_rgbi_palette(),
    )
}

pub(super) fn render_strided_brgi_with_palette(
    source: &[u8],
    width: usize,
    height: usize,
    source_row_bytes: usize,
    plane_stride: usize,
    palette: &[[u8; 3]; DIGITAL_RGBI_COLOR_COUNT],
) -> Result<RgbImage> {
    ensure!(
        width > 0 && width.is_multiple_of(8) && height > 0,
        "B/R/G/I dimensions must be positive and byte aligned"
    );
    let width_bytes = width / 8;
    ensure!(
        width_bytes <= source_row_bytes,
        "B/R/G/I width exceeds its source row stride"
    );
    let source_end = 3usize
        .checked_mul(plane_stride)
        .and_then(|offset| offset.checked_add((height - 1) * source_row_bytes))
        .and_then(|offset| offset.checked_add(width_bytes))
        .context("B/R/G/I source boundary overflow")?;
    ensure!(
        source_end <= source.len(),
        "B/R/G/I source ends at {source_end:#x}, past {:#x}",
        source.len()
    );

    let mut pixels = vec![0; width * height * 3];
    for y in 0..height {
        for x in 0..width {
            let mask = 0x80 >> (x % 8);
            let byte = y * source_row_bytes + x / 8;
            let color = (0..4).fold(0usize, |color, plane| {
                color | (usize::from(source[plane * plane_stride + byte] & mask != 0) << plane)
            });
            pixels[(y * width + x) * 3..(y * width + x + 1) * 3].copy_from_slice(&palette[color]);
        }
    }
    Ok(RgbImage {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
#[path = "planar_tests.rs"]
mod planar_tests;
