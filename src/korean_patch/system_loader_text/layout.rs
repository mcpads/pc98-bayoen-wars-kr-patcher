use anyhow::{Context, Result, ensure};

pub(super) fn paragraph_aligned_end(minimum_end: usize) -> Result<usize> {
    minimum_end
        .checked_add(15)
        .map(|end| end & !15)
        .context("MEGDOS.SYS resident layout size overflow")
}

pub(super) fn reconstruct_resident_extension(
    source: &[u8],
    source_resident_end: usize,
    output_resident_end: usize,
) -> Result<Vec<u8>> {
    ensure!(
        source_resident_end < output_resident_end
            && source_resident_end.is_multiple_of(16)
            && output_resident_end.is_multiple_of(16)
            && source_resident_end < source.len(),
        "MEGDOS.SYS resident extension has invalid paragraph boundaries"
    );
    let extension_byte_count = output_resident_end - source_resident_end;
    let output_size = source
        .len()
        .checked_add(extension_byte_count)
        .context("MEGDOS.SYS reconstructed size overflow")?;
    let mut output = Vec::with_capacity(output_size);
    output.extend_from_slice(&source[..source_resident_end]);
    output.resize(output_resident_end, 0);
    output.extend_from_slice(&source[source_resident_end..]);
    ensure!(
        output.len() == output_size
            && output[output_resident_end..] == source[source_resident_end..],
        "MEGDOS.SYS tail changed while inserting the resident extension"
    );
    Ok(output)
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
