use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use serde::Serialize;

const FILE_NAME: &str = "IO98.SYS";
const FILE_SIZE: usize = 9_523;
const ENTRY_FILE_OFFSET: usize = 0x18a0;
const DIAGNOSTIC_ENTRY_FILE_OFFSET: usize = 0x0a74;
const DIAGNOSTIC_OUTPUT_LOOP_FILE_OFFSET: usize = 0x186d;

const DIAGNOSTICS: [(usize, &str); 14] = [
    (0x03fe, "\r\nInterrupt trap HALT"),
    (0x0414, "\r\nAX="),
    (0x041a, "BX="),
    (0x041e, "CX="),
    (0x0422, "DX="),
    (0x0426, "SP="),
    (0x042a, "BP="),
    (0x042e, "SI="),
    (0x0432, "DI="),
    (0x0436, "\r\nDS="),
    (0x043c, "ES="),
    (0x0440, "SS="),
    (0x0444, "CS="),
    (0x0448, "IP="),
];

const FLAG_LABELS: [(usize, &str, usize, &str); 8] = [
    (0x044c, "OV", 0x045c, "NV"),
    (0x044e, "DN", 0x045e, "UP"),
    (0x0450, "EI", 0x0460, "DI"),
    (0x0452, "NG", 0x0462, "PL"),
    (0x0454, "ZR", 0x0464, "NZ"),
    (0x0456, "AC", 0x0466, "NA"),
    (0x0458, "PE", 0x0468, "PO"),
    (0x045a, "CY", 0x046a, "NC"),
];

const DEVICE_HEADERS: [(usize, &str, u16, u16, u16); 4] = [
    (0x1d6a, "CON     ", 0x8003, 0x050c, 0x051d),
    (0x1d7c, "AUX     ", 0x8000, 0x050c, 0x0523),
    (0x1d8e, "PRN     ", 0x8000, 0x050c, 0x0529),
    (0x1da0, "CLOCK   ", 0x8008, 0x050c, 0x052f),
];

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SystemIoDiagnostic {
    pub file_offset: usize,
    pub text: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SystemIoFlagLabels {
    pub set_file_offset: usize,
    pub set_text: String,
    pub clear_file_offset: usize,
    pub clear_text: String,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SystemIoDevice {
    pub file_offset: usize,
    pub name: String,
    pub attributes: u16,
    pub strategy_offset: u16,
    pub interrupt_offset: u16,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct SystemIoCatalog {
    pub name: String,
    pub byte_size: usize,
    pub entry_file_offset: usize,
    pub diagnostic_entry_file_offset: usize,
    pub diagnostic_output_loop_file_offset: usize,
    pub diagnostics: Vec<SystemIoDiagnostic>,
    pub flag_labels: Vec<SystemIoFlagLabels>,
    pub devices: Vec<SystemIoDevice>,
}

pub(super) fn catalog_system_io(boot_files: &BTreeMap<String, Vec<u8>>) -> Result<SystemIoCatalog> {
    let bytes = boot_files
        .get(FILE_NAME)
        .with_context(|| format!("preserved boot population is missing {FILE_NAME}"))?;
    ensure!(
        bytes.len() == FILE_SIZE,
        "{FILE_NAME} does not match the complete supported system image size"
    );
    ensure_bytes(bytes, 0, &[0xe9, 0x9d, 0x18], "initial entry jump")?;
    ensure_bytes(
        bytes,
        ENTRY_FILE_OFFSET,
        &[0xfa, 0x8c, 0xc8, 0x8e, 0xd0, 0xbc, 0x00, 0x02],
        "hardware initialization entry",
    )?;
    ensure_bytes(
        bytes,
        DIAGNOSTIC_ENTRY_FILE_OFFSET,
        &[0x0e, 0x1f, 0xbb, 0xfe, 0x03, 0xe8, 0xf1, 0x0d],
        "interrupt diagnostic entry",
    )?;
    ensure_bytes(
        bytes,
        DIAGNOSTIC_OUTPUT_LOOP_FILE_OFFSET,
        &[
            0x8a, 0x07, 0x43, 0x84, 0xc0, 0x74, 0x07, 0x53, 0xe8, 0xeb, 0xff, 0x5b, 0xeb, 0xf2,
            0xc3,
        ],
        "null-terminated diagnostic output loop",
    )?;

    let diagnostics = DIAGNOSTICS
        .iter()
        .map(|&(file_offset, expected)| {
            require_null_terminated_ascii(bytes, file_offset, expected)?;
            Ok(SystemIoDiagnostic {
                file_offset,
                text: expected.to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let flag_labels = FLAG_LABELS
        .iter()
        .map(
            |&(set_file_offset, set_text, clear_file_offset, clear_text)| {
                ensure_bytes(
                    bytes,
                    set_file_offset,
                    set_text.as_bytes(),
                    "set-state flag label",
                )?;
                ensure_bytes(
                    bytes,
                    clear_file_offset,
                    clear_text.as_bytes(),
                    "clear-state flag label",
                )?;
                Ok(SystemIoFlagLabels {
                    set_file_offset,
                    set_text: set_text.to_owned(),
                    clear_file_offset,
                    clear_text: clear_text.to_owned(),
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;
    let devices = DEVICE_HEADERS
        .iter()
        .map(
            |&(file_offset, name, attributes, strategy_offset, interrupt_offset)| {
                ensure!(
                    read_u16(bytes, file_offset + 4)? == attributes,
                    "{FILE_NAME} {name:?} device attributes do not match"
                );
                ensure!(
                    read_u16(bytes, file_offset + 6)? == strategy_offset,
                    "{FILE_NAME} {name:?} strategy offset does not match"
                );
                ensure!(
                    read_u16(bytes, file_offset + 8)? == interrupt_offset,
                    "{FILE_NAME} {name:?} interrupt offset does not match"
                );
                ensure_bytes(
                    bytes,
                    file_offset + 10,
                    name.as_bytes(),
                    "built-in device name",
                )?;
                Ok(SystemIoDevice {
                    file_offset,
                    name: name.trim_end().to_owned(),
                    attributes,
                    strategy_offset,
                    interrupt_offset,
                })
            },
        )
        .collect::<Result<Vec<_>>>()?;

    Ok(SystemIoCatalog {
        name: FILE_NAME.to_owned(),
        byte_size: bytes.len(),
        entry_file_offset: ENTRY_FILE_OFFSET,
        diagnostic_entry_file_offset: DIAGNOSTIC_ENTRY_FILE_OFFSET,
        diagnostic_output_loop_file_offset: DIAGNOSTIC_OUTPUT_LOOP_FILE_OFFSET,
        diagnostics,
        flag_labels,
        devices,
    })
}

fn require_null_terminated_ascii(bytes: &[u8], offset: usize, expected: &str) -> Result<()> {
    ensure!(
        expected.is_ascii(),
        "declared {FILE_NAME} diagnostic is not ASCII"
    );
    ensure_bytes(bytes, offset, expected.as_bytes(), "diagnostic text")?;
    ensure!(
        bytes.get(offset + expected.len()) == Some(&0),
        "{FILE_NAME} diagnostic at {offset:#x} is not null-terminated"
    );
    Ok(())
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let raw: [u8; 2] = bytes
        .get(offset..offset + 2)
        .with_context(|| format!("truncated {FILE_NAME} word at {offset:#x}"))?
        .try_into()
        .expect("a two-byte range converts to an array");
    Ok(u16::from_le_bytes(raw))
}

fn ensure_bytes(bytes: &[u8], offset: usize, expected: &[u8], role: &str) -> Result<()> {
    ensure!(
        bytes.get(offset..offset + expected.len()) == Some(expected),
        "{FILE_NAME} {role} does not match at file offset {offset:#x}"
    );
    Ok(())
}

#[cfg(test)]
#[path = "system_io_tests.rs"]
mod system_io_tests;
