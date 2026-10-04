use std::collections::BTreeSet;
use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use crate::source_disk::sha256_hex;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) struct RgbSheet {
    pub(super) width: usize,
    pub(super) height: usize,
    pub(super) pixels: Vec<u8>,
}

pub(super) fn load_rgb_sheet(
    path: &Path,
    expected_sha256: &str,
    expected_width: usize,
    expected_height: usize,
) -> Result<RgbSheet> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read Arle artwork {}", path.display()))?;
    ensure!(
        sha256_hex(&bytes) == expected_sha256,
        "Arle artwork hash differs from its manifest"
    );
    let sheet = decode_rgb_png(&bytes)?;
    ensure!(
        sheet.width == expected_width && sheet.height == expected_height,
        "Arle artwork dimensions differ from its manifest"
    );
    Ok(sheet)
}

pub(super) fn validate_declared_palette(sheet: &RgbSheet, palette: &[[u8; 3]]) -> Result<usize> {
    let colors = sheet
        .pixels
        .as_chunks::<3>()
        .0
        .iter()
        .map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect::<BTreeSet<_>>();
    ensure!(
        colors.iter().all(|color| palette.contains(color)),
        "normalized Arle artwork contains a color outside its declared source palette"
    );
    Ok(colors.len())
}

fn decode_rgb_png(bytes: &[u8]) -> Result<RgbSheet> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().context("read Arle artwork PNG")?;
    let mut buffer = vec![
        0;
        reader
            .output_buffer_size()
            .context("Arle artwork PNG output buffer size overflow")?
    ];
    let output = reader
        .next_frame(&mut buffer)
        .context("decode Arle artwork PNG")?;
    let source = &buffer[..output.buffer_size()];
    let pixel_count = usize::try_from(output.width)? * usize::try_from(output.height)?;
    let mut pixels = Vec::with_capacity(pixel_count * 3);
    match output.color_type {
        png::ColorType::Rgb => pixels.extend_from_slice(source),
        png::ColorType::Rgba => {
            for pixel in source.as_chunks::<4>().0 {
                ensure!(pixel[3] == u8::MAX, "Arle artwork PNG must be fully opaque");
                pixels.extend_from_slice(&pixel[..3]);
            }
        }
        other => anyhow::bail!("Arle artwork PNG decoded as unsupported {other:?}"),
    }
    ensure!(
        pixels.len() == pixel_count * 3,
        "Arle artwork RGB population is incomplete"
    );
    Ok(RgbSheet {
        width: usize::try_from(output.width)?,
        height: usize::try_from(output.height)?,
        pixels,
    })
}

#[cfg(test)]
#[path = "raster_tests.rs"]
mod raster_tests;
