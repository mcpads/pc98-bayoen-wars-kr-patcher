use std::collections::BTreeMap;

use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::model::CompiledMenuTextEntry;
use crate::game_data::{MenuTextCatalog, MenuTextEntry};
use crate::korean_patch::shared_text::{CompiledGaijiBank, encode_dos_character};
use crate::source_disk::sha256_hex;
use crate::translation_drafts::{
    DevelopmentPolicy, TranslationDraftEntry, TranslationDraftSegment,
};

const DOS_TERMINATOR: u8 = b'$';
const AT_TERMINATOR: u8 = b'@';
const BELL: u8 = 0x07;
const DYNAMIC_ENTRY_ID: &str = "menu-text-031";
const POSITION_FIELD_ID: &str = "menu-floppy-position-field";
const DRIVE_LABEL_FIELD_ID: &str = "menu-floppy-drive-label-field";
const DYNAMIC_PREFIX: &[u8] = b"\\P\0\0\\C6";
const DYNAMIC_SUFFIX: &[u8] = b" \\C7";

pub(super) fn compile_records(
    program: &[u8],
    source: &MenuTextCatalog,
    draft: &TranslationDraftSegment,
    bank: &CompiledGaijiBank,
) -> Result<Vec<CompiledMenuTextEntry>> {
    ensure!(
        source.entries.len() == draft.entries.len(),
        "MENU source and draft populations differ"
    );
    source
        .entries
        .iter()
        .zip(&draft.entries)
        .map(|(source_entry, draft_entry)| compile_record(program, source_entry, draft_entry, bank))
        .collect()
}

fn compile_record(
    program: &[u8],
    source: &MenuTextEntry,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<CompiledMenuTextEntry> {
    ensure!(
        source.id == draft.id,
        "MENU source and draft identities differ"
    );
    let terminator = source_terminator(source)?;
    let body = program
        .get(source.file_offset..source.file_offset + source.byte_size)
        .with_context(|| format!("{} source body lies outside MENU.COM", source.id))?;
    ensure!(
        program.get(source.file_offset + source.byte_size) == Some(&terminator),
        "{} source terminator changed",
        source.id
    );
    ensure!(
        sha256_hex(body) == source.raw_sha256,
        "{} source body hash changed",
        source.id
    );
    let decoded = SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(body)
        .with_context(|| format!("{} source body is not valid Shift_JIS", source.id))?;
    ensure!(
        decoded.replace('@', "\n") == source.text,
        "{} source bytes no longer match its protected text",
        source.id
    );

    let (bytes, runtime_field_offsets) = match draft.development_policy {
        DevelopmentPolicy::Translate => {
            validate_draft_lines(draft)?;
            if source.id == DYNAMIC_ENTRY_ID {
                encode_dynamic_record(source, body, terminator, draft, bank)?
            } else {
                (
                    encode_structured_record(source, body, terminator, draft, bank)?,
                    BTreeMap::new(),
                )
            }
        }
        DevelopmentPolicy::PreserveSourceControl => {
            ensure!(
                draft.korean_text.is_empty(),
                "{} control-only draft gained visible text",
                draft.id
            );
            let mut bytes = body.to_vec();
            bytes.push(terminator);
            (bytes, BTreeMap::new())
        }
        DevelopmentPolicy::RetainSourceAudio => {
            anyhow::bail!("{} uses an audio-only development policy", draft.id)
        }
    };
    Ok(CompiledMenuTextEntry {
        id: source.id.clone(),
        original_file_offset: source.file_offset,
        file_offset: 0,
        com_address: 0,
        lines: draft.korean_text.clone(),
        bytes,
        runtime_field_offsets,
    })
}

fn validate_draft_lines(draft: &TranslationDraftEntry) -> Result<()> {
    ensure!(
        !draft.korean_text.is_empty()
            && draft.korean_text.iter().all(|line| {
                !line.is_empty()
                    && !line.contains(DOS_TERMINATOR as char)
                    && !line.contains(AT_TERMINATOR as char)
                    && !line.chars().any(char::is_control)
            }),
        "{} draft contains an empty line, control, or reserved terminator",
        draft.id
    );
    Ok(())
}

fn encode_structured_record(
    source: &MenuTextEntry,
    body: &[u8],
    terminator: u8,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<Vec<u8>> {
    let mut translated = draft.korean_text.iter();
    let mut output = Vec::new();
    let mut cursor = 0;
    let mut line_start = 0;
    while cursor < body.len() {
        match body[cursor] {
            b'\r' => {
                ensure!(
                    body.get(cursor + 1) == Some(&b'\n'),
                    "{} source has a lone carriage return",
                    source.id
                );
                encode_line(
                    source,
                    &body[line_start..cursor],
                    &mut translated,
                    bank,
                    &mut output,
                )?;
                output.extend_from_slice(b"\r\n");
                cursor += 2;
                line_start = cursor;
            }
            b'\n' => anyhow::bail!("{} source has a lone line feed", source.id),
            AT_TERMINATOR => {
                encode_line(
                    source,
                    &body[line_start..cursor],
                    &mut translated,
                    bank,
                    &mut output,
                )?;
                output.push(AT_TERMINATOR);
                cursor += 1;
                line_start = cursor;
            }
            b'\\' | 0 => anyhow::bail!(
                "{} has an unsupported embedded command outside the dynamic record",
                source.id
            ),
            BELL => cursor += 1,
            _ => cursor += shift_jis_character_size(body, cursor, &source.id)?,
        }
    }
    encode_line(
        source,
        &body[line_start..],
        &mut translated,
        bank,
        &mut output,
    )?;
    ensure!(
        translated.next().is_none(),
        "{} draft has more lines than its visible source slots",
        source.id
    );
    output.push(terminator);
    Ok(output)
}

fn encode_line<'a>(
    source: &MenuTextEntry,
    source_line: &[u8],
    translated: &mut impl Iterator<Item = &'a String>,
    bank: &CompiledGaijiBank,
    output: &mut Vec<u8>,
) -> Result<()> {
    let Some(first_visible) = source_line.iter().position(|byte| *byte != BELL) else {
        output.extend_from_slice(source_line);
        return Ok(());
    };
    let last_visible = source_line
        .iter()
        .rposition(|byte| *byte != BELL)
        .expect("a visible source line has a final byte");
    ensure!(
        !source_line[first_visible..=last_visible].contains(&BELL),
        "{} source has a BELL embedded inside visible text",
        source.id
    );
    output.extend_from_slice(&source_line[..first_visible]);
    let line = translated
        .next()
        .with_context(|| format!("{} draft has fewer lines than its source", source.id))?;
    encode_text(line, bank, output)?;
    output.extend_from_slice(&source_line[last_visible + 1..]);
    Ok(())
}

fn encode_dynamic_record(
    source: &MenuTextEntry,
    body: &[u8],
    terminator: u8,
    draft: &TranslationDraftEntry,
    bank: &CompiledGaijiBank,
) -> Result<(Vec<u8>, BTreeMap<String, usize>)> {
    ensure!(
        terminator == DOS_TERMINATOR
            && body.starts_with(DYNAMIC_PREFIX)
            && body.ends_with(DYNAMIC_SUFFIX),
        "{} dynamic control structure changed",
        source.id
    );
    ensure!(
        draft.korean_text.len() == 1,
        "{} dynamic record requires exactly one draft line",
        source.id
    );
    let line = &draft.korean_text[0];
    let (before, after) = line
        .split_once("??")
        .context("MENU dynamic drive-label draft lost its ?? runtime field")?;
    ensure!(
        !before.contains("??") && !after.contains("??") && !after.contains('?'),
        "MENU dynamic drive-label draft has more than one runtime field"
    );

    let mut output = DYNAMIC_PREFIX.to_vec();
    encode_text(before, bank, &mut output)?;
    let drive_label_offset = output.len();
    output.extend_from_slice(b"??");
    encode_text(after, bank, &mut output)?;
    output.extend_from_slice(DYNAMIC_SUFFIX);
    output.push(terminator);
    let runtime_field_offsets = BTreeMap::from([
        (POSITION_FIELD_ID.to_owned(), 2),
        (DRIVE_LABEL_FIELD_ID.to_owned(), drive_label_offset),
    ]);
    Ok((output, runtime_field_offsets))
}

fn encode_text(text: &str, bank: &CompiledGaijiBank, output: &mut Vec<u8>) -> Result<()> {
    for character in text.chars() {
        output.extend(encode_dos_character(character, bank)?);
    }
    Ok(())
}

fn source_terminator(source: &MenuTextEntry) -> Result<u8> {
    match source.terminator_hex.as_str() {
        "24" => Ok(DOS_TERMINATOR),
        "40" => Ok(AT_TERMINATOR),
        _ => anyhow::bail!("{} has an unsupported source terminator", source.id),
    }
}

fn shift_jis_character_size(bytes: &[u8], offset: usize, id: &str) -> Result<usize> {
    let first = bytes[offset];
    if matches!(first, 0x81..=0x9f | 0xe0..=0xfc) {
        let second = *bytes
            .get(offset + 1)
            .with_context(|| format!("{id} ends inside a Shift_JIS character"))?;
        ensure!(
            matches!(second, 0x40..=0x7e | 0x80..=0xfc) && second != 0x7f,
            "{id} has an invalid Shift_JIS trail byte"
        );
        Ok(2)
    } else {
        ensure!(
            matches!(first, 0x20..=0x7e | 0xa1..=0xdf),
            "{id} has an unsupported source byte {first:#04x}"
        );
        Ok(1)
    }
}

#[cfg(test)]
#[path = "records_tests.rs"]
mod records_tests;
