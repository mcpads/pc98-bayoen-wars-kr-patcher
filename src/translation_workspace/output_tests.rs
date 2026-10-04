use super::*;
use crate::translation_workspace::model::{
    ProtectedWorkspaceIndex, SourceSegmentReference, TargetFileCoverage, TargetFileDisposition,
};

#[test]
fn index_keeps_population_completion_separate_from_segmentation() {
    let directory = tempfile::tempdir().unwrap();
    let index = ProtectedWorkspaceIndex {
        supported_source_sha256: "source".to_owned(),
        target_population_complete: true,
        translation_segmentation_complete: false,
        target_files: vec![TargetFileCoverage {
            source_file: "TITLE.DAT".to_owned(),
            disposition: TargetFileDisposition::NeedsSourceSegmentation,
            segment_ids: Vec::new(),
            evidence: vec!["visible title text".to_owned()],
        }],
        segments: vec![SourceSegmentReference {
            id: "menu".to_owned(),
            source_file: "MENU.COM".to_owned(),
            surface: "menu_text".to_owned(),
            path: "segments/menu.json".to_owned(),
            entry_count: 1,
            content_sha256: "segment".to_owned(),
        }],
    };

    write_index(directory.path(), &index).unwrap();

    let parsed: serde_json::Value =
        serde_json::from_slice(&fs::read(directory.path().join("index.json")).unwrap()).unwrap();
    assert_eq!(parsed["target_population_complete"], true);
    assert_eq!(parsed["translation_segmentation_complete"], false);
}
