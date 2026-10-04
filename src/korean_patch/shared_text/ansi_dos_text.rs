use anyhow::{Context, Result, ensure};
use encoding_rs::SHIFT_JIS;

use super::{CompiledGaijiBank, encode_dos_character};

const DOS_TERMINATOR: u8 = b'$';
const ESCAPE: u8 = 0x1b;
const FULL_WIDTH_SPACE: [u8; 2] = [0x81, 0x40];

#[derive(Clone, Copy, Eq, PartialEq)]
enum TokenKind {
    Ansi,
    Whitespace,
    Visible,
}

#[derive(Clone, Copy)]
struct Token {
    start: usize,
    end: usize,
    kind: TokenKind,
}

/// Replaces only visible Shift_JIS text while preserving the exact ANSI,
/// indentation, decoration-only lines, CRLF boundaries, and DOS terminator
/// contract observed in an external program string.
pub(crate) fn encode_ansi_dos_dollar_text(
    source_text: &str,
    source_bytes: &[u8],
    korean_lines: &[String],
    bank: &CompiledGaijiBank,
) -> Result<Vec<u8>> {
    ensure!(
        !source_bytes.contains(&DOS_TERMINATOR),
        "ANSI DOS source body contains its terminator"
    );
    let decoded = SHIFT_JIS
        .decode_without_bom_handling_and_without_replacement(source_bytes)
        .context("ANSI DOS source body is not valid Shift_JIS")?;
    ensure!(
        decoded.as_ref() == source_text,
        "ANSI DOS source bytes no longer match the protected decoded text"
    );
    ensure!(
        korean_lines.iter().all(|line| {
            !line.is_empty()
                && !line.contains(DOS_TERMINATOR as char)
                && !line.chars().any(char::is_control)
        }),
        "ANSI DOS draft contains an empty line, control, or string terminator"
    );

    let mut translated = korean_lines.iter();
    let mut output = Vec::new();
    let mut cursor = 0;
    while cursor < source_bytes.len() {
        let (body_end, boundary_end) = next_line(source_bytes, cursor)?;
        let body = &source_bytes[cursor..body_end];
        let tokens = tokenize_line(body)?;
        let visible = tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Visible)
            .collect::<Vec<_>>();
        if visible.is_empty() || is_decoration_only(body, &visible)? {
            output.extend_from_slice(body);
        } else {
            let content_start = visible.first().expect("visible line is non-empty").start;
            let content_end = visible.last().expect("visible line is non-empty").end;
            output.extend_from_slice(&body[..content_start]);
            let line = translated
                .next()
                .context("ANSI DOS draft has fewer lines than its visible source slots")?;
            for character in line.chars() {
                output.extend(encode_dos_character(character, bank)?);
            }
            output.extend_from_slice(&body[content_end..]);
        }
        output.extend_from_slice(&source_bytes[body_end..boundary_end]);
        cursor = boundary_end;
    }
    ensure!(
        translated.next().is_none(),
        "ANSI DOS draft has more lines than its visible source slots"
    );
    output.push(DOS_TERMINATOR);
    Ok(output)
}

fn next_line(bytes: &[u8], start: usize) -> Result<(usize, usize)> {
    let mut cursor = start;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\r' => {
                ensure!(
                    bytes.get(cursor + 1) == Some(&b'\n'),
                    "ANSI DOS source contains a lone carriage return"
                );
                return Ok((cursor, cursor + 2));
            }
            b'\n' => anyhow::bail!("ANSI DOS source contains a lone line feed"),
            _ => cursor += 1,
        }
    }
    Ok((bytes.len(), bytes.len()))
}

fn tokenize_line(line: &[u8]) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let start = cursor;
        let kind = if line[cursor] == ESCAPE {
            cursor = ansi_end(line, cursor)?;
            TokenKind::Ansi
        } else if line[cursor] == b' ' || line[cursor] == b'\t' {
            cursor += 1;
            TokenKind::Whitespace
        } else if line.get(cursor..cursor + 2) == Some(FULL_WIDTH_SPACE.as_slice()) {
            cursor += 2;
            TokenKind::Whitespace
        } else {
            cursor += shift_jis_character_size(line, cursor)?;
            TokenKind::Visible
        };
        tokens.push(Token {
            start,
            end: cursor,
            kind,
        });
    }
    Ok(tokens)
}

fn ansi_end(line: &[u8], start: usize) -> Result<usize> {
    ensure!(
        line.get(start + 1) == Some(&b'['),
        "ANSI DOS source contains an unsupported escape sequence"
    );
    let mut cursor = start + 2;
    while let Some(&byte) = line.get(cursor) {
        cursor += 1;
        if (0x40..=0x7e).contains(&byte) {
            return Ok(cursor);
        }
        ensure!(
            (0x20..=0x3f).contains(&byte),
            "ANSI DOS source contains an invalid CSI byte"
        );
    }
    anyhow::bail!("ANSI DOS source contains a truncated CSI sequence")
}

fn shift_jis_character_size(bytes: &[u8], offset: usize) -> Result<usize> {
    let first = bytes[offset];
    if matches!(first, 0x81..=0x9f | 0xe0..=0xfc) {
        let second = *bytes
            .get(offset + 1)
            .context("ANSI DOS source ends inside a Shift_JIS character")?;
        ensure!(
            matches!(second, 0x40..=0x7e | 0x80..=0xfc),
            "ANSI DOS source contains an invalid Shift_JIS trail byte"
        );
        Ok(2)
    } else {
        ensure!(
            matches!(first, 0x20..=0x7e | 0xa1..=0xdf),
            "ANSI DOS source contains an unsupported byte {first:#04x}"
        );
        Ok(1)
    }
}

fn is_decoration_only(line: &[u8], visible: &[&Token]) -> Result<bool> {
    let mut characters = Vec::with_capacity(visible.len());
    for token in visible {
        let character = SHIFT_JIS
            .decode_without_bom_handling_and_without_replacement(&line[token.start..token.end])
            .context("ANSI DOS decoration token is not valid Shift_JIS")?;
        characters.extend(character.chars());
    }
    Ok(!characters.is_empty() && characters.iter().all(|character| *character == '～'))
}

#[cfg(test)]
#[path = "ansi_dos_text_tests.rs"]
mod ansi_dos_text_tests;
