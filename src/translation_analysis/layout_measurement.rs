use anyhow::{Context, Result};
use encoding_rs::SHIFT_JIS;
use serde_json::Value;

use super::model::{TranslationLayoutFinding, TranslationLayoutFindingKind};
use crate::translation_drafts::{TranslationDraftEntry, TranslationSurface};

const FIXED_GRID_LINES: usize = 6;
const FIXED_GRID_COLUMNS: usize = 7;
const MONOCHROME_PAGE_LINES: usize = 4;
const MONOCHROME_PAGE_COLUMNS: usize = 16;
const DEVELOPMENT_FONT_CELL_PIXELS: usize = 16;

pub(super) struct MeasuredLayoutEntry {
    pub source_line_count: usize,
    pub korean_line_count: usize,
    pub max_source_line_cells: usize,
    pub max_korean_line_cells: usize,
    pub findings: Vec<TranslationLayoutFinding>,
}

pub(super) fn measure_layout_entry(
    segment_id: &str,
    surface: TranslationSurface,
    protected: &Value,
    draft: &TranslationDraftEntry,
) -> Result<MeasuredLayoutEntry> {
    let source_lines = source_lines(surface, protected)?;
    let korean_lines = &draft.korean_text;
    let mut findings = Vec::new();
    for (line_index, line) in korean_lines.iter().enumerate() {
        if line.contains(['\r', '\n']) {
            findings.push(finding(
                TranslationLayoutFindingKind::EmbeddedLineBreak,
                segment_id,
                &draft.id,
                Some(line_index),
                1,
                0,
            ));
        }
    }
    match surface {
        TranslationSurface::FixedGaijiText => measure_grid(
            segment_id,
            &draft.id,
            korean_lines,
            FIXED_GRID_LINES,
            FIXED_GRID_COLUMNS,
            TranslationLayoutFindingKind::FixedGridLineOverflow,
            TranslationLayoutFindingKind::FixedGridColumnOverflow,
            &mut findings,
        ),
        TranslationSurface::OpeningGlyphIndexPages | TranslationSurface::EndingGlyphIndexPages => {
            measure_grid(
                segment_id,
                &draft.id,
                korean_lines,
                MONOCHROME_PAGE_LINES,
                MONOCHROME_PAGE_COLUMNS,
                TranslationLayoutFindingKind::MonochromePageLineOverflow,
                TranslationLayoutFindingKind::MonochromePageColumnOverflow,
                &mut findings,
            )
        }
        TranslationSurface::BakedGraphicsText => {
            let (line_limit, column_limit) = baked_region_limits(protected)?;
            measure_grid(
                segment_id,
                &draft.id,
                korean_lines,
                line_limit,
                column_limit,
                TranslationLayoutFindingKind::BakedRegionLineOverflow,
                TranslationLayoutFindingKind::BakedRegionColumnOverflow,
                &mut findings,
            );
        }
        _ => {}
    }

    if korean_lines.len() > source_lines.len() && !source_lines.is_empty() {
        findings.push(finding(
            TranslationLayoutFindingKind::SourceLineCountGrowth,
            segment_id,
            &draft.id,
            None,
            korean_lines.len(),
            source_lines.len(),
        ));
    }
    for (line_index, korean) in korean_lines.iter().enumerate() {
        if let Some(source) = source_lines.get(line_index) {
            let source_cells = visible_cells(source);
            let korean_cells = visible_cells(korean);
            if korean_cells > source_cells {
                findings.push(finding(
                    TranslationLayoutFindingKind::SourceLineWidthGrowth,
                    segment_id,
                    &draft.id,
                    Some(line_index),
                    korean_cells,
                    source_cells,
                ));
            }
        }
    }
    if is_runtime_string_surface(surface)
        && let (Some(source_text), Some(source_byte_size)) = (
            protected.get("text").and_then(Value::as_str),
            protected.get("byte_size").and_then(Value::as_u64),
        )
    {
        let visible_source = strip_ansi_sequences(source_text);
        let source_visible_bytes = encoded_development_len(&visible_source);
        let preserved_overhead = usize::try_from(source_byte_size)
            .unwrap_or(usize::MAX)
            .saturating_sub(source_visible_bytes);
        let separator = if source_text.contains("\r\n") {
            "\r\n"
        } else {
            "\n"
        };
        let replacement = korean_lines.join(separator);
        let required = preserved_overhead + encoded_development_len(&replacement);
        let source_capacity = usize::try_from(source_byte_size).unwrap_or(usize::MAX);
        if required > source_capacity {
            findings.push(finding(
                TranslationLayoutFindingKind::SourceStorageGrowth,
                segment_id,
                &draft.id,
                None,
                required,
                source_capacity,
            ));
        }
    }

    Ok(MeasuredLayoutEntry {
        source_line_count: source_lines.len(),
        korean_line_count: korean_lines.len(),
        max_source_line_cells: source_lines
            .iter()
            .map(|line| visible_cells(line))
            .max()
            .unwrap_or(0),
        max_korean_line_cells: korean_lines
            .iter()
            .map(|line| visible_cells(line))
            .max()
            .unwrap_or(0),
        findings,
    })
}

fn source_lines(surface: TranslationSurface, protected: &Value) -> Result<Vec<String>> {
    match surface {
        TranslationSurface::FixedGaijiText
        | TranslationSurface::OpeningGlyphIndexPages
        | TranslationSurface::EndingGlyphIndexPages => protected
            .get("lines")
            .and_then(Value::as_array)
            .context("protected fixed layout has no lines")?
            .iter()
            .map(|line| {
                let cells = line
                    .as_array()
                    .context("protected fixed layout line is not an array")?;
                Ok("x".repeat(cells.len()))
            })
            .collect(),
        TranslationSurface::BakedGraphicsText => Ok(Vec::new()),
        _ => {
            let text = protected
                .get("text")
                .and_then(Value::as_str)
                .context("protected runtime layout has no text")?;
            Ok(strip_ansi_sequences(text)
                .replace('\r', "")
                .split('\n')
                .map(str::to_owned)
                .collect())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn measure_grid(
    segment_id: &str,
    entry_id: &str,
    lines: &[String],
    line_limit: usize,
    column_limit: usize,
    line_kind: TranslationLayoutFindingKind,
    column_kind: TranslationLayoutFindingKind,
    findings: &mut Vec<TranslationLayoutFinding>,
) {
    if lines.len() > line_limit {
        findings.push(finding(
            line_kind,
            segment_id,
            entry_id,
            None,
            lines.len(),
            line_limit,
        ));
    }
    for (line_index, line) in lines.iter().enumerate() {
        let cells = visible_cells(line);
        if cells > column_limit {
            findings.push(finding(
                column_kind,
                segment_id,
                entry_id,
                Some(line_index),
                cells,
                column_limit,
            ));
        }
    }
}

fn baked_region_limits(protected: &Value) -> Result<(usize, usize)> {
    let regions = protected
        .get("screen_regions")
        .and_then(Value::as_array)
        .context("protected baked layout has no screen regions")?;
    let line_limit = regions
        .iter()
        .map(|region| {
            region
                .get("height")
                .and_then(Value::as_u64)
                .context("baked screen region has no height")
                .and_then(|height| usize::try_from(height).context("region height overflow"))
                .map(|height| height / DEVELOPMENT_FONT_CELL_PIXELS)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .min()
        .context("protected baked layout has no screen region")?;
    let column_limit = regions
        .iter()
        .map(|region| {
            region
                .get("width")
                .and_then(Value::as_u64)
                .context("baked screen region has no width")
                .and_then(|width| usize::try_from(width).context("region width overflow"))
                .map(|width| width / DEVELOPMENT_FONT_CELL_PIXELS)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .min()
        .context("protected baked layout has no screen region")?;
    Ok((line_limit, column_limit))
}

fn visible_cells(text: &str) -> usize {
    text.chars()
        .filter(|character| !character.is_control())
        .count()
}

fn strip_ansi_sequences(text: &str) -> String {
    let mut stripped = String::new();
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' && characters.peek() == Some(&'[') {
            characters.next();
            for parameter in characters.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&parameter) {
                    break;
                }
            }
        } else {
            stripped.push(character);
        }
    }
    stripped
}

fn encoded_development_len(text: &str) -> usize {
    text.chars()
        .map(|character| {
            let mut encoded = [0_u8; 4];
            let value = character.encode_utf8(&mut encoded);
            let (bytes, _, had_errors) = SHIFT_JIS.encode(value);
            if had_errors { 2 } else { bytes.len() }
        })
        .sum()
}

fn is_runtime_string_surface(surface: TranslationSurface) -> bool {
    matches!(
        surface,
        TranslationSurface::DosSystemText
            | TranslationSurface::InterfaceText
            | TranslationSurface::BattleCallout
            | TranslationSurface::Dialogue
            | TranslationSurface::MenuText
            | TranslationSurface::ExternalProgramText
    )
}

fn finding(
    kind: TranslationLayoutFindingKind,
    segment_id: &str,
    entry_id: &str,
    line_index: Option<usize>,
    measured: usize,
    limit: usize,
) -> TranslationLayoutFinding {
    TranslationLayoutFinding {
        kind,
        segment_id: segment_id.to_owned(),
        entry_id: entry_id.to_owned(),
        line_index,
        measured,
        limit,
    }
}

#[cfg(test)]
#[path = "layout_measurement_tests.rs"]
mod layout_measurement_tests;
