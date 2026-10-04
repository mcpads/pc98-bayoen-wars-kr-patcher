use anyhow::{Context, Result, ensure};

const COM_ORIGIN: usize = 0x100;

pub(super) fn ensure_prefix(
    bytes: &[u8],
    offset: usize,
    expected: &[u8],
    label: &str,
) -> Result<()> {
    ensure!(
        bytes.get(offset..offset + expected.len()) == Some(expected),
        "{label} does not match at file offset {offset:#x}"
    );
    Ok(())
}

pub(super) fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("truncated 16-bit field at file offset {offset:#x}"))?
        .try_into()
        .expect("a two-byte range converts to a two-byte array");
    Ok(u16::from_le_bytes(raw))
}

pub(super) fn com_address_to_file_offset(address: usize) -> Result<usize> {
    address
        .checked_sub(COM_ORIGIN)
        .with_context(|| format!("COM address {address:#x} lies below the {COM_ORIGIN:#x} origin"))
}
