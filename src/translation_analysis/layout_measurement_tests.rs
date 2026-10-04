use serde_json::json;

use super::*;
use crate::translation_drafts::{DevelopmentPolicy, DraftStatus};

#[test]
fn fixed_grid_reports_only_actual_cell_overflow() {
    let protected = json!({
        "lines": [[], [], [], [], [], []]
    });
    let draft = draft(["일곱글자까지다!", "짧음"]);

    let measurement = measure_layout_entry(
        "fixed",
        TranslationSurface::FixedGaijiText,
        &protected,
        &draft,
    )
    .unwrap();

    assert!(measurement.findings.iter().any(|finding| {
        finding.kind == TranslationLayoutFindingKind::FixedGridColumnOverflow
            && finding.line_index == Some(0)
    }));
    assert!(
        !measurement
            .findings
            .iter()
            .any(|finding| { finding.kind == TranslationLayoutFindingKind::FixedGridLineOverflow })
    );
}

#[test]
fn runtime_storage_growth_preserves_control_overhead() {
    let protected = json!({
        "text": "\u{1b}[24;0H短い",
        "byte_size": 12
    });
    let draft = draft(["아주 긴 한국어 문장"]);

    let measurement = measure_layout_entry(
        "system",
        TranslationSurface::DosSystemText,
        &protected,
        &draft,
    )
    .unwrap();

    assert!(measurement.findings.iter().any(|finding| {
        finding.kind == TranslationLayoutFindingKind::SourceStorageGrowth
            && finding.measured > finding.limit
    }));
}

#[test]
fn baked_region_uses_the_smallest_shared_variant() {
    let protected = json!({
        "screen_regions": [
            {"width": 64, "height": 32},
            {"width": 48, "height": 32}
        ]
    });
    let draft = draft(["네글자다"]);

    let measurement = measure_layout_entry(
        "baked",
        TranslationSurface::BakedGraphicsText,
        &protected,
        &draft,
    )
    .unwrap();

    assert!(measurement.findings.iter().any(|finding| {
        finding.kind == TranslationLayoutFindingKind::BakedRegionColumnOverflow
            && finding.limit == 3
    }));
}

fn draft<const N: usize>(lines: [&str; N]) -> TranslationDraftEntry {
    TranslationDraftEntry {
        id: "entry".to_owned(),
        korean_text: lines.into_iter().map(str::to_owned).collect(),
        development_policy: DevelopmentPolicy::Translate,
        status: DraftStatus::NeedsHumanReview,
        notes: None,
        questions: Vec::new(),
    }
}
