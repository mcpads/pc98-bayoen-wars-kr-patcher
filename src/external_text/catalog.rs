use std::collections::BTreeMap;

use anyhow::{Context, Result};
use serde::Serialize;

use super::mouse_driver::catalog_mouse_driver;
use super::shell::catalog_shell;
use super::sound_drivers::catalog_sound_drivers;
use super::system_loader::catalog_system_loader;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalTextStorage {
    OriginalFile,
    UnpackedComImage,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalTextTerminator {
    DosDollar,
    Null,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ExternalTextCatalog {
    pub program_count: usize,
    pub unique_text_count: usize,
    pub reference_count: usize,
    pub programs: Vec<ExternalProgramTextCatalog>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ExternalProgramTextCatalog {
    pub file_name: String,
    pub storage: ExternalTextStorage,
    pub packed_size: usize,
    pub text_storage_size: usize,
    pub unique_text_count: usize,
    pub reference_count: usize,
    pub entries: Vec<ExternalTextEntry>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ExternalTextEntry {
    pub id: String,
    pub text_offset: usize,
    pub runtime_address: usize,
    pub terminator: ExternalTextTerminator,
    pub byte_size: usize,
    pub raw_sha256: String,
    pub raw_hex: String,
    pub text: String,
    pub references: Vec<ExternalTextReference>,
}

#[derive(Debug, Eq, PartialEq, Serialize)]
pub struct ExternalTextReference {
    pub role: String,
    pub consumer_offset: usize,
    pub table_index: Option<usize>,
}

pub(crate) fn catalog_external_text(
    installer_payload: &BTreeMap<String, Vec<u8>>,
    runtime_files: &BTreeMap<String, Vec<u8>>,
    boot_files: &BTreeMap<String, Vec<u8>>,
) -> Result<ExternalTextCatalog> {
    let mut programs = catalog_sound_drivers(
        required(installer_payload, "BPLAY6.COM")?,
        required(installer_payload, "FPLAY6.COM")?,
        required(runtime_files, "BSAMP.COM")?,
    )?;
    programs.push(catalog_shell(required(boot_files, "DSH.COM")?)?);
    programs.push(catalog_mouse_driver(required(
        runtime_files,
        "NMOUSE.COM",
    )?)?);
    programs.push(catalog_system_loader(required(boot_files, "MEGDOS.SYS")?)?);
    programs.sort_by(|left, right| left.file_name.cmp(&right.file_name));

    Ok(ExternalTextCatalog {
        program_count: programs.len(),
        unique_text_count: programs
            .iter()
            .map(|program| program.unique_text_count)
            .sum(),
        reference_count: programs.iter().map(|program| program.reference_count).sum(),
        programs,
    })
}

fn required<'a>(files: &'a BTreeMap<String, Vec<u8>>, name: &str) -> Result<&'a [u8]> {
    files
        .get(name)
        .map(Vec::as_slice)
        .with_context(|| format!("external text source is missing {name}"))
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod catalog_tests;
