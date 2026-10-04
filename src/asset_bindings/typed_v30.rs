use std::ops::Range;

use anyhow::{Context, Result, ensure};
use v30::{Instruction, decode_bytes};

pub(super) fn decode_complete_block(
    program: &[u8],
    range: Range<usize>,
    role: &str,
) -> Result<Vec<Instruction>> {
    let block = program
        .get(range.clone())
        .with_context(|| format!("MAD.COM {role} block lies outside the file"))?;
    let mut instructions = Vec::new();
    let mut offset = 0;
    while offset < block.len() {
        let decoded = decode_bytes(&block[offset..]).with_context(|| {
            format!(
                "MAD.COM {role} is not valid typed V30 code at file offset {:#x}",
                range.start + offset
            )
        })?;
        ensure!(decoded.byte_len > 0, "typed V30 decoder made no progress");
        offset += decoded.byte_len;
        instructions.push(decoded.instruction);
    }
    ensure!(
        offset == block.len(),
        "MAD.COM {role} ends inside an instruction"
    );
    ensure!(
        matches!(instructions.last(), Some(Instruction::Ret { pop: 0 })),
        "MAD.COM {role} does not end in a typed near RET"
    );
    Ok(instructions)
}

#[cfg(test)]
#[path = "typed_v30_tests.rs"]
mod typed_v30_tests;
