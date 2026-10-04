use std::fs;

use serde::Serialize;
use serde_json::json;

use super::*;
use crate::source_disk::sha256_hex;

#[derive(Serialize)]
struct FixtureEntry {
    id: String,
    korean_text: Vec<String>,
    development_policy: &'static str,
    status: &'static str,
    notes: Option<String>,
    questions: Vec<String>,
}

#[test]
fn corpus_requires_the_reviewed_entry_hash() {
    let directory = tempfile::tempdir().unwrap();
    write_fixture(directory.path());
    load_translation_corpus(directory.path()).unwrap();

    let segment_path = directory.path().join("segments/interface.json");
    let mut segment: serde_json::Value =
        serde_json::from_slice(&fs::read(&segment_path).unwrap()).unwrap();
    segment["entries"][0]["korean_text"] = json!(["변조"]);
    fs::write(&segment_path, serde_json::to_vec_pretty(&segment).unwrap()).unwrap();

    let error = load_translation_corpus(directory.path()).err().unwrap();
    assert!(error.to_string().contains("reviewed draft hash"));
}

fn write_fixture(directory: &std::path::Path) {
    fs::create_dir(directory.join("segments")).unwrap();
    let entries = vec![FixtureEntry {
        id: "interface-01".to_owned(),
        korean_text: vec!["한글".to_owned()],
        development_policy: "translate",
        status: "needs_human_review",
        notes: None,
        questions: Vec::new(),
    }];
    let draft_sha256 = sha256_hex(&serde_json::to_vec(&entries).unwrap());
    let segment = json!({
        "id": "interface",
        "surface": "interface_text",
        "protected_segment_sha256": "protected",
        "draft_sha256": draft_sha256,
        "entries": entries,
        "review": {
            "reviewed_draft_sha256": draft_sha256,
            "method": "independent_second_llm",
            "outcome": "human_decision_required",
            "notes": ["독립 대조"]
        }
    });
    fs::write(
        directory.join("segments/interface.json"),
        serde_json::to_vec_pretty(&segment).unwrap(),
    )
    .unwrap();
    let index = json!({
        "supported_source_sha256": "source",
        "protected_index_sha256": "protected-index",
        "status": "needs_human_review",
        "translation_unit_count": 1,
        "translated_text_unit_count": 1,
        "preserved_source_control_unit_count": 0,
        "retained_source_audio_unit_count": 0,
        "producer_evidence_segments": ["opening-glyph-atlas"],
        "segments": [{
            "id": "interface",
            "surface": "interface_text",
            "path": "segments/interface.json",
            "entry_count": 1,
            "protected_segment_sha256": "protected",
            "draft_sha256": draft_sha256
        }]
    });
    fs::write(
        directory.join("index.json"),
        serde_json::to_vec_pretty(&index).unwrap(),
    )
    .unwrap();
}
