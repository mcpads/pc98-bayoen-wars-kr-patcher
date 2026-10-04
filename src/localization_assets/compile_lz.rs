use anyhow::{Context, Result, bail, ensure};

const MINIMUM_COPY_SIZE: usize = 3;
const MAXIMUM_COPY_SIZE: usize = 130;
const MAXIMUM_COPY_DISTANCE: usize = 256;
const MAXIMUM_LITERAL_SIZE: usize = 0x7f;

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct DecodedStream {
    pub packed_offset: usize,
    pub packed_size: usize,
    pub command_count: usize,
    pub output: Vec<u8>,
}

pub(crate) fn decode_all_streams(input: &[u8]) -> Result<Vec<DecodedStream>> {
    let mut streams = Vec::new();
    let mut packed_offset = 0usize;

    while packed_offset < input.len() {
        let stream = decode_stream(&input[packed_offset..], packed_offset)?;
        packed_offset += stream.packed_size;
        streams.push(stream);
    }

    if streams.is_empty() {
        bail!("Compile LZ asset contains no streams");
    }
    Ok(streams)
}

pub(crate) fn encode_single_stream(input: &[u8]) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut cursor = 0usize;
    let mut literal_start = 0usize;

    while cursor < input.len() {
        let (copy_size, distance) = longest_copy(input, cursor);
        if copy_size >= MINIMUM_COPY_SIZE {
            append_literals(&mut output, &input[literal_start..cursor]);
            output.push(
                0x80 | u8::try_from(copy_size - MINIMUM_COPY_SIZE)
                    .expect("Compile LZ copy size fits its command"),
            );
            output.push(
                u8::try_from(distance - 1).expect("Compile LZ copy distance fits its command"),
            );
            cursor += copy_size;
            literal_start = cursor;
        } else {
            cursor += 1;
        }
    }
    append_literals(&mut output, &input[literal_start..]);
    output.push(0);

    let decoded = decode_all_streams(&output)?;
    ensure!(
        decoded.len() == 1 && decoded[0].packed_size == output.len(),
        "Compile LZ encoder did not produce exactly one complete stream"
    );
    ensure!(
        decoded[0].output == input,
        "Compile LZ encoder failed its decoder round trip"
    );
    Ok(output)
}

fn longest_copy(input: &[u8], cursor: usize) -> (usize, usize) {
    let maximum_size = (input.len() - cursor).min(MAXIMUM_COPY_SIZE);
    if maximum_size < MINIMUM_COPY_SIZE {
        return (0, 0);
    }
    let mut best_size = 0usize;
    let mut best_distance = 0usize;
    for distance in 1..=cursor.min(MAXIMUM_COPY_DISTANCE) {
        let mut size = 0usize;
        while size < maximum_size && input[cursor + size] == input[cursor + size - distance] {
            size += 1;
        }
        if size > best_size {
            best_size = size;
            best_distance = distance;
            if size == maximum_size {
                break;
            }
        }
    }
    (best_size, best_distance)
}

fn append_literals(output: &mut Vec<u8>, mut literals: &[u8]) {
    while !literals.is_empty() {
        let size = literals.len().min(MAXIMUM_LITERAL_SIZE);
        output.push(u8::try_from(size).expect("Compile LZ literal size fits its command"));
        output.extend_from_slice(&literals[..size]);
        literals = &literals[size..];
    }
}

fn decode_stream(input: &[u8], packed_offset: usize) -> Result<DecodedStream> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    let mut command_count = 0usize;

    loop {
        let command_offset = packed_offset + cursor;
        let command = *input.get(cursor).with_context(|| {
            format!("Compile LZ stream at 0x{packed_offset:X} has no terminator")
        })?;
        cursor += 1;
        command_count += 1;

        if command == 0 {
            return Ok(DecodedStream {
                packed_offset,
                packed_size: cursor,
                command_count,
                output,
            });
        }

        if command < 0x80 {
            let literal_size = usize::from(command);
            let end = cursor
                .checked_add(literal_size)
                .context("Compile LZ literal range overflow")?;
            let literal = input.get(cursor..end).with_context(|| {
                format!("Compile LZ literal at 0x{command_offset:X} exceeds the packed input")
            })?;
            output.extend_from_slice(literal);
            cursor = end;
            continue;
        }

        let copy_size = usize::from(command & 0x7f) + 3;
        let distance = usize::from(*input.get(cursor).with_context(|| {
            format!("Compile LZ back-reference at 0x{command_offset:X} has no distance")
        })?) + 1;
        cursor += 1;

        for _ in 0..copy_size {
            let value = output
                .len()
                .checked_sub(distance)
                .and_then(|source| output.get(source).copied())
                .unwrap_or(0);
            output.push(value);
        }
    }
}

#[cfg(test)]
#[path = "compile_lz_tests.rs"]
mod compile_lz_tests;
