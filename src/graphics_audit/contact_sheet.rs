use anyhow::{Result, ensure};

use super::planar::RgbImage;

const BACKGROUND: [u8; 3] = [32, 32, 32];

pub(super) fn compose_contact_sheet(
    images: Vec<RgbImage>,
    maximum_width: usize,
    gap: usize,
) -> Result<RgbImage> {
    ensure!(!images.is_empty(), "contact sheet has no images");
    ensure!(maximum_width > 0, "contact sheet width is zero");
    ensure!(
        images.iter().all(|image| image.width <= maximum_width),
        "contact sheet contains an image wider than its canvas"
    );

    let mut placements = Vec::with_capacity(images.len());
    let mut x = 0usize;
    let mut y = 0usize;
    let mut row_height = 0usize;
    let mut used_width = 0usize;
    for image in &images {
        if x > 0 && x + image.width > maximum_width {
            y += row_height + gap;
            x = 0;
            row_height = 0;
        }
        placements.push((x, y));
        used_width = used_width.max(x + image.width);
        row_height = row_height.max(image.height);
        x += image.width + gap;
    }
    let height = y + row_height;
    let mut pixels = BACKGROUND.repeat(used_width * height);
    for (image, (destination_x, destination_y)) in images.into_iter().zip(placements) {
        for row in 0..image.height {
            let source_start = row * image.width * 3;
            let destination_start = ((destination_y + row) * used_width + destination_x) * 3;
            pixels[destination_start..destination_start + image.width * 3]
                .copy_from_slice(&image.pixels[source_start..source_start + image.width * 3]);
        }
    }

    Ok(RgbImage {
        width: used_width,
        height,
        pixels,
    })
}

#[cfg(test)]
#[path = "contact_sheet_tests.rs"]
mod contact_sheet_tests;
