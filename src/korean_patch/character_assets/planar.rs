use anyhow::{Context, Result, ensure};

pub(super) fn decode_compact_brgi_indices(
    source: &[u8],
    width: usize,
    height: usize,
) -> Result<Vec<u8>> {
    let (width_bytes, plane_size, expected_size) = geometry(width, height)?;
    ensure!(
        source.len() == expected_size,
        "compact B/R/G/I source size differs from its frame geometry"
    );
    let mut indices = vec![0; width * height];
    for y in 0..height {
        for x in 0..width {
            let mask = 0x80 >> (x % 8);
            let byte = y * width_bytes + x / 8;
            indices[y * width + x] = (0..4).fold(0u8, |index, plane| {
                index | (u8::from(source[plane * plane_size + byte] & mask != 0) << plane)
            });
        }
    }
    Ok(indices)
}

pub(super) fn encode_compact_brgi_indices(
    indices: &[u8],
    width: usize,
    height: usize,
) -> Result<Vec<u8>> {
    let (width_bytes, plane_size, expected_size) = geometry(width, height)?;
    ensure!(
        indices.len() == width * height,
        "indexed character frame size differs from its geometry"
    );
    ensure!(
        indices.iter().all(|index| *index < 16),
        "indexed character frame contains a palette index above 15"
    );
    let mut output = vec![0; expected_size];
    for y in 0..height {
        for x in 0..width {
            let mask = 0x80 >> (x % 8);
            let byte = y * width_bytes + x / 8;
            let index = indices[y * width + x];
            for plane in 0..4 {
                if index & (1 << plane) != 0 {
                    output[plane * plane_size + byte] |= mask;
                }
            }
        }
    }
    Ok(output)
}

fn geometry(width: usize, height: usize) -> Result<(usize, usize, usize)> {
    ensure!(
        width > 0 && width.is_multiple_of(8) && height > 0,
        "compact B/R/G/I dimensions must be positive and byte aligned"
    );
    let width_bytes = width / 8;
    let plane_size = width_bytes
        .checked_mul(height)
        .context("compact B/R/G/I plane size overflow")?;
    let expected_size = plane_size
        .checked_mul(4)
        .context("compact B/R/G/I frame size overflow")?;
    Ok((width_bytes, plane_size, expected_size))
}

#[cfg(test)]
#[path = "planar_tests.rs"]
mod planar_tests;
