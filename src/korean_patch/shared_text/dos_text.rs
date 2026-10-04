use anyhow::{Result, ensure};

use super::{CompiledGaijiBank, encode_dos_character};

const DOS_TERMINATOR: u8 = b'$';

pub(crate) fn encode_dos_dollar_text(
    source_text: &str,
    korean_lines: &[String],
    bank: &CompiledGaijiBank,
) -> Result<Vec<u8>> {
    ensure!(!korean_lines.is_empty(), "DOS text has no Korean lines");
    let source_body = source_text
        .strip_suffix("\r\n")
        .ok_or_else(|| anyhow::anyhow!("DOS source text lost its final CRLF"))?;
    let source_lines = source_body.split("\r\n").collect::<Vec<_>>();
    let source_visible_line_count = source_lines
        .iter()
        .take_while(|line| !line.is_empty())
        .count();
    ensure!(
        source_lines[source_visible_line_count..]
            .iter()
            .all(|line| line.is_empty()),
        "DOS source text contains an unsupported internal blank line"
    );
    ensure!(
        source_visible_line_count == korean_lines.len(),
        "DOS text changed its visible line count"
    );
    ensure!(
        korean_lines
            .iter()
            .all(|line| !line.is_empty() && !line.contains(DOS_TERMINATOR as char)),
        "DOS text contains an empty line or the string terminator"
    );

    let mut bytes = Vec::new();
    for line in korean_lines {
        for character in line.chars() {
            bytes.extend(encode_dos_character(character, bank)?);
        }
        bytes.extend_from_slice(b"\r\n");
    }
    for _ in source_visible_line_count..source_lines.len() {
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.push(DOS_TERMINATOR);
    Ok(bytes)
}

pub(crate) fn encode_dos_dollar_text_layout(
    source_text: &str,
    korean_lines: &[String],
    bank: &CompiledGaijiBank,
) -> Result<Vec<u8>> {
    ensure!(!korean_lines.is_empty(), "DOS text has no Korean lines");
    let source_lines = source_text.split("\r\n").collect::<Vec<_>>();
    ensure!(
        source_lines.iter().all(|line| !line.contains(['\r', '\n'])),
        "DOS source text contains a non-CRLF line boundary"
    );
    ensure!(
        source_lines.iter().filter(|line| !line.is_empty()).count() == korean_lines.len(),
        "DOS text changed its visible line count"
    );
    ensure!(
        korean_lines.iter().all(|line| {
            !line.is_empty()
                && !line.contains(DOS_TERMINATOR as char)
                && !line.contains(['\r', '\n'])
        }),
        "DOS text contains an empty line, line boundary, or string terminator"
    );

    let mut translated = korean_lines.iter();
    let mut bytes = Vec::new();
    for (index, source_line) in source_lines.iter().enumerate() {
        if !source_line.is_empty() {
            let line = translated
                .next()
                .expect("visible source population was checked above");
            for character in line.chars() {
                bytes.extend(encode_dos_character(character, bank)?);
            }
        }
        if index + 1 < source_lines.len() {
            bytes.extend_from_slice(b"\r\n");
        }
    }
    ensure!(
        translated.next().is_none(),
        "DOS text has unused Korean lines"
    );
    bytes.push(DOS_TERMINATOR);
    Ok(bytes)
}

#[cfg(test)]
#[path = "dos_text_tests.rs"]
mod dos_text_tests;
