use anyhow::{Context, Result, ensure};

use super::contact_sheet::compose_contact_sheet;
use super::planar::{RgbImage, render_strided_brgi};

pub(super) fn render_masked_brgi_sheet(
    source: &[u8],
    record_offset: usize,
    record_count: usize,
    width: usize,
    height: usize,
    maximum_width: usize,
    gap: usize,
) -> Result<RgbImage> {
    ensure!(
        width > 0 && width.is_multiple_of(8) && height > 0 && record_count > 0,
        "invalid masked B/R/G/I record geometry"
    );
    let width_bytes = width / 8;
    let plane_size = width_bytes
        .checked_mul(height)
        .context("masked record plane size overflow")?;
    let record_size = plane_size * 5;
    let source_end = record_offset
        .checked_add(record_count * record_size)
        .context("masked record boundary overflow")?;
    ensure!(
        source_end <= source.len(),
        "masked records exceed their source asset"
    );

    let images = (0..record_count)
        .map(|record_index| {
            let colors = record_offset + record_index * record_size + plane_size;
            render_strided_brgi(
                &source[colors..colors + plane_size * 4],
                width,
                height,
                width_bytes,
                plane_size,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    compose_contact_sheet(images, maximum_width, gap)
}

#[cfg(test)]
#[path = "masked_sheet_tests.rs"]
mod masked_sheet_tests;
