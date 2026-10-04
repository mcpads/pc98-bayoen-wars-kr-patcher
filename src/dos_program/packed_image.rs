use anyhow::{Context, Result, ensure};

const DECOMPRESSOR_SIZE: usize = 0x71;
const MAXIMUM_COM_IMAGE_SIZE: usize = 0x1_0000;

const DECOMPRESSOR: [u8; DECOMPRESSOR_SIZE] = [
    0xfc, 0x60, 0x8c, 0xc8, 0x8e, 0xd8, 0x80, 0xc4, 0x10, 0x8e, 0xc0, 0xbe, 0x00, 0x01, 0x8b, 0xfe,
    0xb9, 0x80, 0x7f, 0xf3, 0xa5, 0x0e, 0x06, 0x68, 0x1b, 0x01, 0xcb, 0x8c, 0xd8, 0x05, 0x10, 0x00,
    0x8e, 0xc0, 0x8c, 0xc8, 0x8e, 0xd8, 0xbe, 0x71, 0x01, 0x33, 0xff, 0xac, 0x0a, 0xc0, 0x74, 0x36,
    0x78, 0x06, 0x8a, 0xc8, 0xf3, 0xa4, 0xeb, 0xf3, 0x25, 0x7f, 0x00, 0x05, 0x03, 0x00, 0x8b, 0xc8,
    0xac, 0x40, 0x8b, 0xde, 0x8c, 0xda, 0x8b, 0xf7, 0x2b, 0xf0, 0x8c, 0xc0, 0x8e, 0xd8, 0x72, 0x08,
    0xf3, 0xa4, 0x8b, 0xf3, 0x8e, 0xda, 0xeb, 0xd3, 0x8a, 0xc5, 0xaa, 0x46, 0xe0, 0xfc, 0x74, 0xf0,
    0x8b, 0xf3, 0x8e, 0xda, 0xeb, 0xc5, 0x58, 0x8e, 0xc0, 0x8e, 0xd8, 0x61, 0x1e, 0x68, 0x00, 0x01,
    0xcb,
];

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct PackedComImage {
    pub(crate) packed_size: usize,
    pub(crate) unpacked: Vec<u8>,
}

pub(crate) fn unpack_self_expanding_com(bytes: &[u8]) -> Result<PackedComImage> {
    ensure!(
        bytes.get(..DECOMPRESSOR_SIZE) == Some(DECOMPRESSOR.as_slice()),
        "DOS COM program does not use the supported self-expanding stub"
    );
    let packed = bytes
        .get(DECOMPRESSOR_SIZE..)
        .context("self-expanding DOS COM program is missing its packed stream")?;
    let (unpacked, consumed) = decode_packed_stream(packed)?;
    ensure!(
        consumed == packed.len(),
        "self-expanding DOS COM program has {} trailing bytes after its packed stream",
        packed.len() - consumed
    );

    Ok(PackedComImage {
        packed_size: bytes.len(),
        unpacked,
    })
}

/// Builds the self-expanding COM representation consumed by the verified
/// decompressor stub.
///
/// The observed stream format permits several encodings for the same output.
/// The producer uses a deterministic longest-match search over the complete
/// 256-byte window consumed by the target stub. Equal-length matches prefer the
/// nearest source. Literal and match command limits come directly from the
/// verified decoder above.
pub(crate) fn pack_self_expanding_com(unpacked: &[u8]) -> Result<Vec<u8>> {
    ensure!(
        unpacked.len() <= MAXIMUM_COM_IMAGE_SIZE,
        "unpacked DOS COM image exceeds {MAXIMUM_COM_IMAGE_SIZE} bytes"
    );

    let maximum_literal_commands = unpacked.len().div_ceil(0x7f);
    let capacity = DECOMPRESSOR_SIZE
        .checked_add(unpacked.len())
        .and_then(|size| size.checked_add(maximum_literal_commands))
        .and_then(|size| size.checked_add(1))
        .context("packed DOS COM size overflow")?;
    let mut packed = Vec::with_capacity(capacity);
    packed.extend_from_slice(&DECOMPRESSOR);
    let mut cursor = 0;
    let mut literal_start = 0;
    while cursor < unpacked.len() {
        if let Some((length, distance)) = longest_match(unpacked, cursor) {
            emit_literals(&mut packed, &unpacked[literal_start..cursor]);
            packed.push(0x80 | u8::try_from(length - 3).expect("match length fits in u8"));
            packed.push(u8::try_from(distance - 1).expect("match distance fits in u8"));
            cursor += length;
            literal_start = cursor;
        } else {
            cursor += 1;
            if cursor - literal_start == 0x7f {
                emit_literals(&mut packed, &unpacked[literal_start..cursor]);
                literal_start = cursor;
            }
        }
    }
    emit_literals(&mut packed, &unpacked[literal_start..]);
    packed.push(0);
    Ok(packed)
}

fn longest_match(unpacked: &[u8], cursor: usize) -> Option<(usize, usize)> {
    let maximum_length = (unpacked.len() - cursor).min(0x82);
    let maximum_distance = cursor.min(0x100);
    let mut best = None;
    for distance in 1..=maximum_distance {
        let mut length = 0;
        while length < maximum_length
            && unpacked[cursor + length] == unpacked[cursor + length - distance]
        {
            length += 1;
        }
        if length >= 3 && best.is_none_or(|(best_length, _)| length > best_length) {
            best = Some((length, distance));
        }
    }
    best
}

fn emit_literals(packed: &mut Vec<u8>, literal: &[u8]) {
    for chunk in literal.chunks(0x7f) {
        packed.push(u8::try_from(chunk.len()).expect("literal chunks fit in u8"));
        packed.extend_from_slice(chunk);
    }
}

fn decode_packed_stream(packed: &[u8]) -> Result<(Vec<u8>, usize)> {
    let mut cursor = 0;
    let mut unpacked = Vec::new();

    loop {
        let command = *packed
            .get(cursor)
            .with_context(|| format!("packed DOS COM stream has no terminator at {cursor:#x}"))?;
        cursor += 1;
        if command == 0 {
            return Ok((unpacked, cursor));
        }

        if command < 0x80 {
            let length = usize::from(command);
            let literal = packed.get(cursor..cursor + length).with_context(|| {
                format!("truncated {length}-byte DOS COM literal at {cursor:#x}")
            })?;
            ensure_output_capacity(unpacked.len(), length)?;
            unpacked.extend_from_slice(literal);
            cursor += length;
            continue;
        }

        let length = usize::from(command & 0x7f) + 3;
        let distance = usize::from(*packed.get(cursor).with_context(|| {
            format!(
                "DOS COM back-reference at {:#x} has no distance",
                cursor - 1
            )
        })?) + 1;
        cursor += 1;
        ensure_output_capacity(unpacked.len(), length)?;
        for _ in 0..length {
            let value = unpacked
                .len()
                .checked_sub(distance)
                .and_then(|source| unpacked.get(source).copied())
                .unwrap_or(0);
            unpacked.push(value);
        }
    }
}

fn ensure_output_capacity(current: usize, additional: usize) -> Result<()> {
    let end = current
        .checked_add(additional)
        .context("unpacked DOS COM size overflow")?;
    ensure!(
        end <= MAXIMUM_COM_IMAGE_SIZE,
        "unpacked DOS COM image exceeds {MAXIMUM_COM_IMAGE_SIZE} bytes"
    );
    Ok(())
}

#[cfg(test)]
#[path = "packed_image_tests.rs"]
mod packed_image_tests;
