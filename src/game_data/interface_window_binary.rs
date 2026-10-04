use anyhow::{Context, Result, ensure};
use v30::{CallTarget, Instruction, Operand, Register16, decode_bytes};

pub(super) const COM_ORIGIN: usize = 0x100;
pub(super) const WINDOW_STATE_TABLE_OFFSET: usize = 0x6ac8;
pub(super) const WINDOW_FRAME_RECORD_SIZE: usize = 12;
const GRAPHICS_ROW_BYTES: u16 = 80;
const PIXELS_PER_GRAPHICS_BYTE: u16 = 8;
const FRAME_INVALIDATION_CONSUMER_FILE_OFFSET: usize = 0x6952;
const FRAME_INVALIDATION_CONSUMER_PREFIX: [u8; 24] = [
    0x8b, 0x1e, 0x16, 0xd8, // mov bx,[d816]: frame X in pixels
    0xa1, 0x18, 0xd8, // mov ax,[d818]: frame Y in pixels
    0xe8, 0x54, 0xcf, // call 39b0: convert pixels to the map-cache cell
    0x8b, 0x0e, 0x1c, 0xd8, // mov cx,[d81c]: inner height
    0x8b, 0x16, 0x1a, 0xd8, // mov dx,[d81a]: inner width
    0x83, 0xc1, 0x02, // add cx,2: include the frame borders
    0x83, 0xc2, 0x02, // add dx,2: include the frame borders
];

pub(super) fn require_mov_reg_imm(
    program: &[u8],
    offset: usize,
    expected_register: Register16,
    expected_value: u16,
    role: &str,
) -> Result<()> {
    let decoded = decode_at(program, offset, role)?;
    ensure!(
        decoded.instruction
            == Instruction::Mov {
                dest: Operand::Reg16(expected_register),
                src: Operand::Imm16(expected_value),
            }
            && decoded.byte_len == 3
            && decoded.prefixes.is_empty(),
        "{role} instruction at {offset:#x} changed"
    );
    Ok(())
}

pub(super) fn require_near_call(program: &[u8], offset: usize, role: &str) -> Result<u16> {
    let decoded = decode_at(program, offset, role)?;
    match decoded.instruction {
        Instruction::Call {
            target: CallTarget::Rel16(displacement),
        } if decoded.byte_len == 3 && decoded.prefixes.is_empty() => {
            Ok(u16::try_from(offset + COM_ORIGIN + decoded.byte_len)?
                .wrapping_add_signed(displacement))
        }
        _ => anyhow::bail!("{role} call at {offset:#x} changed"),
    }
}

pub(super) fn require_state_check(
    program: &[u8],
    offset: usize,
    state_value: u8,
    role: &str,
) -> Result<()> {
    ensure!(
        program.get(offset..offset + 5) == Some([0x80, 0x3e, 0x11, 0xd8, state_value].as_slice()),
        "{role} state check at {offset:#x} changed"
    );
    Ok(())
}

pub(super) fn require_frame_record(
    program: &[u8],
    state_values: &[u8],
    expected_com_address: u16,
    expected_words: [u16; 6],
    role: &str,
) -> Result<usize> {
    for state_value in state_values {
        let table_offset = WINDOW_STATE_TABLE_OFFSET + usize::from(*state_value) * 2;
        ensure!(
            read_u16(program, table_offset, role)? == expected_com_address,
            "{role} state {state_value:#04x} frame target changed"
        );
    }
    let file_offset = usize::from(expected_com_address)
        .checked_sub(COM_ORIGIN)
        .with_context(|| format!("{role} frame record precedes the COM origin"))?;
    let record = program
        .get(file_offset..file_offset + WINDOW_FRAME_RECORD_SIZE)
        .with_context(|| format!("{role} frame record lies outside MAD.COM"))?;
    let words: [u16; 6] = std::array::from_fn(|index| {
        let offset = index * 2;
        u16::from_le_bytes([record[offset], record[offset + 1]])
    });
    ensure!(words == expected_words, "{role} frame record changed");
    Ok(file_offset)
}

pub(super) fn require_frame_invalidation_geometry(
    frame_origin: u16,
    invalidation_x_pixels: u16,
    invalidation_y_pixels: u16,
    role: &str,
) -> Result<()> {
    let expected_x_pixels = (frame_origin % GRAPHICS_ROW_BYTES)
        .checked_mul(PIXELS_PER_GRAPHICS_BYTE)
        .context("interface frame X coordinate overflowed")?;
    let expected_y_pixels = frame_origin / GRAPHICS_ROW_BYTES;
    ensure!(
        invalidation_x_pixels == expected_x_pixels && invalidation_y_pixels == expected_y_pixels,
        "{role} frame origin and background invalidation coordinates diverged"
    );
    Ok(())
}

pub(super) fn require_frame_invalidation_consumer(program: &[u8], role: &str) -> Result<()> {
    ensure!(
        program.get(
            FRAME_INVALIDATION_CONSUMER_FILE_OFFSET
                ..FRAME_INVALIDATION_CONSUMER_FILE_OFFSET
                    + FRAME_INVALIDATION_CONSUMER_PREFIX.len()
        ) == Some(FRAME_INVALIDATION_CONSUMER_PREFIX.as_slice()),
        "{role} background invalidation consumer changed"
    );
    ensure!(
        require_near_call(program, FRAME_INVALIDATION_CONSUMER_FILE_OFFSET + 7, role)? == 0x39b0,
        "{role} background invalidation coordinate converter changed"
    );
    Ok(())
}

fn read_u16(program: &[u8], offset: usize, role: &str) -> Result<u16> {
    let raw: [u8; 2] = program
        .get(offset..offset + 2)
        .with_context(|| format!("{role} word at {offset:#x} lies outside MAD.COM"))?
        .try_into()
        .expect("two bytes convert to one word");
    Ok(u16::from_le_bytes(raw))
}

fn decode_at(program: &[u8], offset: usize, role: &str) -> Result<v30::DecodedInstruction> {
    decode_bytes(
        program
            .get(offset..)
            .with_context(|| format!("{role} instruction at {offset:#x} is outside MAD.COM"))?,
    )
    .with_context(|| format!("{role} instruction at {offset:#x} is not typed V30 code"))
}

#[cfg(test)]
#[path = "interface_window_binary_tests.rs"]
mod interface_window_binary_tests;
