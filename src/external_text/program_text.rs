use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::catalog::{
    ExternalProgramTextCatalog, ExternalTextEntry, ExternalTextReference, ExternalTextStorage,
    ExternalTextTerminator,
};
use crate::byte_string::encode_lower_hex;
use crate::source_disk::sha256_hex;

const COM_ORIGIN: usize = 0x100;

struct PendingText {
    id_override: Option<String>,
    runtime_address: usize,
    terminator: ExternalTextTerminator,
    bytes: Vec<u8>,
    references: Vec<ExternalTextReference>,
}

struct ReferenceInput<'a> {
    text_offset: usize,
    runtime_address: usize,
    terminator: ExternalTextTerminator,
    consumer_offset: usize,
    role: &'a str,
    table_index: Option<usize>,
    id_override: Option<&'a str>,
}

pub(super) struct ProgramTextBuilder<'a> {
    file_name: &'static str,
    storage: ExternalTextStorage,
    packed_size: usize,
    text_bytes: &'a [u8],
    entries: BTreeMap<usize, PendingText>,
}

impl<'a> ProgramTextBuilder<'a> {
    pub(super) fn new(
        file_name: &'static str,
        storage: ExternalTextStorage,
        packed_size: usize,
        text_bytes: &'a [u8],
    ) -> Self {
        Self {
            file_name,
            storage,
            packed_size,
            text_bytes,
            entries: BTreeMap::new(),
        }
    }

    pub(super) fn add_com_dos_reference(
        &mut self,
        runtime_address: usize,
        consumer_offset: usize,
        role: &str,
        table_index: Option<usize>,
    ) -> Result<()> {
        let text_offset = runtime_address.checked_sub(COM_ORIGIN).with_context(|| {
            format!("COM text address {runtime_address:#x} lies below the origin")
        })?;
        self.add_reference(ReferenceInput {
            text_offset,
            runtime_address,
            terminator: ExternalTextTerminator::DosDollar,
            consumer_offset,
            role,
            table_index,
            id_override: None,
        })
    }

    pub(super) fn add_com_dos_reference_with_id(
        &mut self,
        id: &str,
        runtime_address: usize,
        consumer_offset: usize,
        role: &str,
        table_index: Option<usize>,
    ) -> Result<()> {
        let text_offset = runtime_address.checked_sub(COM_ORIGIN).with_context(|| {
            format!("COM text address {runtime_address:#x} lies below the origin")
        })?;
        self.add_reference(ReferenceInput {
            text_offset,
            runtime_address,
            terminator: ExternalTextTerminator::DosDollar,
            consumer_offset,
            role,
            table_index,
            id_override: Some(id),
        })
    }

    pub(super) fn add_null_reference(
        &mut self,
        text_offset: usize,
        consumer_offset: usize,
        role: &str,
    ) -> Result<()> {
        self.add_reference(ReferenceInput {
            text_offset,
            runtime_address: text_offset,
            terminator: ExternalTextTerminator::Null,
            consumer_offset,
            role,
            table_index: None,
            id_override: None,
        })
    }

    fn add_reference(&mut self, input: ReferenceInput<'_>) -> Result<()> {
        let ReferenceInput {
            text_offset,
            runtime_address,
            terminator,
            consumer_offset,
            role,
            table_index,
            id_override,
        } = input;
        let bytes = read_terminated(self.text_bytes, text_offset, terminator)?;
        let reference = ExternalTextReference {
            role: role.to_owned(),
            consumer_offset,
            table_index,
        };
        if let Some(existing) = self.entries.get_mut(&text_offset) {
            ensure!(
                existing.runtime_address == runtime_address
                    && existing.terminator == terminator
                    && existing.bytes == bytes
                    && existing.id_override.as_deref() == id_override,
                "{} text at {text_offset:#x} has conflicting boundaries",
                self.file_name
            );
            existing.references.push(reference);
            return Ok(());
        }

        self.entries.insert(
            text_offset,
            PendingText {
                id_override: id_override.map(str::to_owned),
                runtime_address,
                terminator,
                bytes: bytes.to_vec(),
                references: vec![reference],
            },
        );
        Ok(())
    }

    pub(super) fn finish(self) -> Result<ExternalProgramTextCatalog> {
        let id_namespace = self
            .file_name
            .split_once('.')
            .map_or(self.file_name, |(stem, _)| stem)
            .to_ascii_lowercase();
        let mut generated_index = 0;
        let entries: Vec<_> = self
            .entries
            .into_iter()
            .map(|(text_offset, pending)| {
                let text = SHIFT_JIS
                    .decode_without_bom_handling_and_without_replacement(&pending.bytes)
                    .with_context(|| {
                        format!(
                            "{} contains invalid Shift_JIS text at {text_offset:#x}",
                            self.file_name
                        )
                    })?
                    .into_owned();
                let id = if let Some(id) = pending.id_override {
                    id
                } else {
                    generated_index += 1;
                    format!("external-{id_namespace}-text-{generated_index:03}")
                };
                Ok(ExternalTextEntry {
                    id,
                    text_offset,
                    runtime_address: pending.runtime_address,
                    terminator: pending.terminator,
                    byte_size: pending.bytes.len(),
                    raw_sha256: sha256_hex(&pending.bytes),
                    raw_hex: encode_lower_hex(&pending.bytes),
                    text,
                    references: pending.references,
                })
            })
            .collect::<Result<_>>()?;
        ensure!(
            entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                == entries.len(),
            "{} text IDs are not unique",
            self.file_name
        );
        let reference_count = entries.iter().map(|entry| entry.references.len()).sum();

        Ok(ExternalProgramTextCatalog {
            file_name: self.file_name.to_owned(),
            storage: self.storage,
            packed_size: self.packed_size,
            text_storage_size: self.text_bytes.len(),
            unique_text_count: entries.len(),
            reference_count,
            entries,
        })
    }
}

fn read_terminated(
    bytes: &[u8],
    offset: usize,
    terminator: ExternalTextTerminator,
) -> Result<&[u8]> {
    let marker = match terminator {
        ExternalTextTerminator::DosDollar => b'$',
        ExternalTextTerminator::Null => 0,
    };
    let tail = bytes
        .get(offset..)
        .with_context(|| format!("external text starts outside storage at {offset:#x}"))?;
    let length = tail
        .iter()
        .position(|byte| *byte == marker)
        .with_context(|| {
            format!("external text at {offset:#x} has no {terminator:?} terminator")
        })?;
    Ok(&tail[..length])
}

#[cfg(test)]
#[path = "program_text_tests.rs"]
mod program_text_tests;
