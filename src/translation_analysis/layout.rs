use std::path::Path;

use anyhow::{Result, ensure};

use super::layout_measurement::measure_layout_entry;
use super::load_translation_corpus;
use super::model::{
    TranslationAnalysisStatus, TranslationLayoutAuditReport, TranslationLayoutFindingKind,
    TranslationLayoutOpenGate, TranslationLayoutScopeExclusion, TranslationLayoutSegmentReport,
};
use super::product_scope::excluded_non_game_text_reason;
use super::protected_layout::load_protected_layout_catalog;
use crate::translation_drafts::{DevelopmentPolicy, TranslationSurface};

pub fn audit_translation_layout(
    protected_workspace: &Path,
    translations: &Path,
) -> Result<TranslationLayoutAuditReport> {
    let corpus = load_translation_corpus(translations)?;
    let protected = load_protected_layout_catalog(protected_workspace, &corpus)?;
    let mut segments = Vec::new();
    let mut findings = Vec::new();
    let mut open_consumer_gates = Vec::new();
    let mut scope_exclusions = Vec::new();
    let mut measured_text_unit_count = 0;
    let mut in_game_text_unit_count = 0;
    let mut excluded_non_game_text_unit_count = 0;

    for draft_segment in &corpus.segments {
        let protected_segment = protected.segment(&draft_segment.id);
        ensure!(
            protected_segment.surface == draft_segment.surface,
            "layout segment {} changed surface after protected validation",
            draft_segment.id
        );
        let mut measured_entry_count = 0;
        let mut max_source_line_count = 0;
        let mut max_korean_line_count = 0;
        let mut max_source_line_cells = 0;
        let mut max_korean_line_cells = 0;
        let finding_start = findings.len();
        for (protected_entry, draft_entry) in
            protected_segment.entries.iter().zip(&draft_segment.entries)
        {
            if draft_entry.development_policy != DevelopmentPolicy::Translate {
                continue;
            }
            let measured = measure_layout_entry(
                &draft_segment.id,
                draft_segment.surface,
                protected_entry,
                draft_entry,
            )?;
            measured_entry_count += 1;
            measured_text_unit_count += 1;
            max_source_line_count = max_source_line_count.max(measured.source_line_count);
            max_korean_line_count = max_korean_line_count.max(measured.korean_line_count);
            max_source_line_cells = max_source_line_cells.max(measured.max_source_line_cells);
            max_korean_line_cells = max_korean_line_cells.max(measured.max_korean_line_cells);
            findings.extend(measured.findings);
        }
        if measured_entry_count == 0 {
            continue;
        }
        let segment_findings = &findings[finding_start..];
        segments.push(TranslationLayoutSegmentReport {
            id: draft_segment.id.clone(),
            surface: draft_segment.surface,
            measured_entry_count,
            max_source_line_count,
            max_korean_line_count,
            max_source_line_cells,
            max_korean_line_cells,
            hard_violation_count: segment_findings
                .iter()
                .filter(|finding| finding.kind.is_hard_violation())
                .count(),
            source_growth_advisory_count: segment_findings
                .iter()
                .filter(|finding| is_source_growth(finding.kind))
                .count(),
        });
        let scope_exclusion_reason = excluded_non_game_text_reason(draft_segment.surface);
        if let Some(reason) = scope_exclusion_reason {
            excluded_non_game_text_unit_count += measured_entry_count;
            scope_exclusions.push(TranslationLayoutScopeExclusion {
                segment_id: draft_segment.id.clone(),
                surface: draft_segment.surface,
                affected_entry_count: measured_entry_count,
                reason: reason.to_owned(),
            });
        } else {
            in_game_text_unit_count += measured_entry_count;
        }
        if scope_exclusion_reason.is_none()
            && let Some(reason) =
                in_game_consumer_gate_reason(&draft_segment.id, draft_segment.surface)
        {
            open_consumer_gates.push(TranslationLayoutOpenGate {
                segment_id: draft_segment.id.clone(),
                surface: draft_segment.surface,
                affected_entry_count: measured_entry_count,
                reason: reason.to_owned(),
            });
        }
    }

    let hard_violation_count = findings
        .iter()
        .filter(|finding| finding.kind.is_hard_violation())
        .count();
    let source_growth_advisory_count = findings
        .iter()
        .filter(|finding| is_source_growth(finding.kind))
        .count();
    Ok(TranslationLayoutAuditReport {
        supported_source_sha256: corpus.index.supported_source_sha256,
        status: TranslationAnalysisStatus::NeedsHumanReview,
        translation_unit_count: corpus.index.translation_unit_count,
        measured_text_unit_count,
        in_game_text_unit_count,
        excluded_non_game_text_unit_count,
        hard_layout_fit: hard_violation_count == 0,
        hard_violation_count,
        source_growth_advisory_count,
        open_consumer_gate_count: open_consumer_gates.len(),
        scope_exclusion_count: scope_exclusions.len(),
        segments,
        findings,
        open_consumer_gates,
        scope_exclusions,
    })
}

fn is_source_growth(kind: TranslationLayoutFindingKind) -> bool {
    matches!(
        kind,
        TranslationLayoutFindingKind::SourceLineCountGrowth
            | TranslationLayoutFindingKind::SourceLineWidthGrowth
            | TranslationLayoutFindingKind::SourceStorageGrowth
    )
}

fn in_game_consumer_gate_reason(
    segment_id: &str,
    surface: TranslationSurface,
) -> Option<&'static str> {
    if matches!(
        (segment_id, surface),
        (
            "dialogue-group-01"
                | "dialogue-group-02"
                | "dialogue-group-03"
                | "dialogue-group-04"
                | "dialogue-group-05"
                | "dialogue-group-06"
                | "dialogue-group-07"
                | "dialogue-group-08"
                | "dialogue-group-09"
                | "dialogue-group-10"
                | "dialogue-group-11",
            TranslationSurface::Dialogue
        )
    ) {
        return None;
    }

    match surface {
        TranslationSurface::InterfaceText
        | TranslationSurface::BattleCallout
        | TranslationSurface::Dialogue => Some(
            "static storage and reference readback is complete, but each visible box and runtime reachability still require consumer-path proof",
        ),
        TranslationSurface::OpeningGlyphIndexPages
        | TranslationSurface::EndingGlyphIndexPages
        | TranslationSurface::BakedGraphicsText => None,
        TranslationSurface::FixedGaijiText
        | TranslationSurface::MonochromeGlyphAtlas
        | TranslationSurface::JapaneseVoicePerformance => None,
        TranslationSurface::DosSystemText
        | TranslationSurface::MenuText
        | TranslationSurface::ExternalProgramText => {
            unreachable!("non-game text must be excluded before consumer gate evaluation")
        }
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
