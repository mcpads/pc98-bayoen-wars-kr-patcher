use super::interface_text::parse_interface_text_record;
use super::interface_window_binary::{require_mov_reg_imm, require_near_call};
use super::{GaijiCatalog, InterfaceTextEntry};
use crate::{byte_string::encode_lower_hex, source_disk::sha256_hex};
use anyhow::{Result, ensure};
use serde::Serialize;
use v30::Register16;

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UnitListStatusCatalog {
    pub text_start_offset: usize,
    pub text_end_offset: usize,
    pub entries: Vec<InterfaceTextEntry>,
    pub consumers: Vec<UnitListStatusConsumer>,
}
#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct UnitListStatusConsumer {
    pub text_instruction_offset: usize,
    pub renderer_call_offset: usize,
    pub entry_id: String,
    pub maximum_screen_bytes: usize,
}
pub(super) fn catalog_unit_list_status(
    program: &[u8],
    gaiji: &GaijiCatalog,
) -> Result<UnitListStatusCatalog> {
    let mut entries = Vec::new();
    let mut cursor = 0x68c0;
    while cursor < 0x6940 {
        let parsed = parse_interface_text_record(program, cursor, gaiji)?;
        ensure!(
            parsed.end_offset <= 0x6940,
            "unit list status crosses its pool"
        );
        let raw = &program[cursor..parsed.end_offset];
        entries.push(InterfaceTextEntry {
            id: format!("unit-list-status-{:02}", entries.len() + 1),
            file_offset: cursor,
            com_address: cursor + 0x100,
            byte_size: raw.len(),
            sha256: sha256_hex(raw),
            raw_hex: encode_lower_hex(raw),
            text: parsed.text,
            gaiji_glyph_indices: parsed.gaiji_glyph_indices,
            tokens: parsed.tokens,
        });
        cursor = parsed.end_offset;
    }
    ensure!(
        entries.len() == 10 && cursor == 0x6940,
        "unit list status population changed"
    );
    let sites = [
        0x67f4, 0x67e7, 0x67da, 0x67cd, 0x67c0, 0x6849, 0x683c, 0x682f, 0x6822, 0x6815,
    ];
    let mut consumers = Vec::new();
    for (index, (entry, offset)) in entries.iter().zip(sites).enumerate() {
        require_mov_reg_imm(
            program,
            offset,
            Register16::DX,
            u16::try_from(entry.com_address)?,
            "unit list status address",
        )?;
        ensure!(
            program.get(offset + 3..offset + 5) == Some(&[0xb0, 7]),
            "unit list status color changed"
        );
        ensure!(
            require_near_call(program, offset + 5, "unit list status renderer")? == 0x5728,
            "unit list status renderer changed"
        );
        consumers.push(UnitListStatusConsumer {
            text_instruction_offset: offset,
            renderer_call_offset: offset + 5,
            entry_id: entry.id.clone(),
            maximum_screen_bytes: if index < 5 { 16 } else { 8 },
        });
    }
    Ok(UnitListStatusCatalog {
        text_start_offset: 0x68c0,
        text_end_offset: 0x6940,
        entries,
        consumers,
    })
}
