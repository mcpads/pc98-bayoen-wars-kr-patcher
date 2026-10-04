use super::*;
use crate::{AssetOrigin, LocalizationAssetFile};

fn file(name: &str) -> LocalizationAssetFile {
    LocalizationAssetFile {
        name: name.to_owned(),
        origin: AssetOrigin::InstallerPayload,
        size: 1,
        sha256: name.to_owned(),
        review_status: AssetReviewStatus::ConfirmedLocalizationTarget,
        evidence: vec![format!("{name} evidence")],
    }
}

fn segment(id: &str, source_file: &str) -> SourceSegmentReference {
    SourceSegmentReference {
        id: id.to_owned(),
        source_file: source_file.to_owned(),
        surface: "text".to_owned(),
        path: format!("segments/{id}.json"),
        entry_count: 1,
        content_sha256: id.to_owned(),
    }
}

#[test]
fn each_target_file_gets_one_explicit_workspace_disposition() {
    let names = [
        "MAD.COM",
        "GAIJI.COM",
        "MENU.COM",
        "BPLAY6.COM",
        "BSAMP.COM",
        "DSH.COM",
        "FPLAY6.COM",
        "MEGDOS.SYS",
        "NMOUSE.COM",
        "TITLE.DAT",
        "SEL1.DAT",
        "SEL3.DAT",
        "OPM.DAT",
        "EDM.DAT",
        "SAMPA",
    ];
    let assets = LocalizationAssetCatalog {
        total_files: names.len(),
        confirmed_localization_targets: names.len(),
        evidence_based_exclusions: 0,
        unresolved: 0,
        files: names.into_iter().map(file).collect(),
    };
    let segments = [
        segment("mad", "MAD.COM"),
        segment("menu", "MENU.COM"),
        segment("bplay", "BPLAY6.COM"),
        segment("bsamp", "BSAMP.COM"),
        segment("dsh", "DSH.COM"),
        segment("fplay", "FPLAY6.COM"),
        segment("megdos", "MEGDOS.SYS"),
        segment("nmouse", "NMOUSE.COM"),
        segment("opening-glyph-atlas", "OPM.DAT"),
        segment("ending-glyph-atlas", "EDM.DAT"),
        segment("voice-samples", "SAMPA"),
        segment("title-baked-text", "TITLE.DAT"),
        segment("difficulty-baked-text", "SEL1.DAT"),
        segment("selection-baked-text", "SEL3.DAT"),
    ];

    let coverage = catalog_target_file_coverage(&assets, &segments).unwrap();

    assert_eq!(coverage.len(), 15);
    assert_eq!(
        coverage
            .iter()
            .filter(|file| file.disposition == TargetFileDisposition::SourceUnitsExtracted)
            .count(),
        14
    );
    assert_eq!(
        coverage
            .iter()
            .filter(|file| file.disposition == TargetFileDisposition::FontProducer)
            .count(),
        1
    );
    assert_eq!(
        coverage
            .iter()
            .filter(|file| file.disposition.needs_segmentation())
            .count(),
        0
    );
}

#[test]
fn a_new_target_fails_until_its_workspace_role_is_declared() {
    let assets = LocalizationAssetCatalog {
        total_files: 1,
        confirmed_localization_targets: 1,
        evidence_based_exclusions: 0,
        unresolved: 0,
        files: vec![file("NEW.DAT")],
    };

    let error = catalog_target_file_coverage(&assets, &[]).unwrap_err();

    assert!(
        error
            .to_string()
            .contains("has no translation workspace disposition")
    );
}
