use std::fs;

use serde::Serialize;
use serde_json::json;

use super::*;
use crate::source_disk::sha256_hex;
use crate::translation_analysis::load_translation_corpus;

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
fn protected_entry_identity_must_be_a_string() {
    assert_eq!(entry_id(&json!({"id": "entry-01"})).unwrap(), "entry-01");
    assert!(entry_id(&json!({"id": 1})).is_err());
    assert!(entry_id(&json!({})).is_err());
}

#[test]
fn protected_layout_catalog_requires_the_reviewed_source_hashes() {
    let directory = tempfile::tempdir().unwrap();
    let protected = directory.path().join("protected");
    let translations = directory.path().join("translations");
    write_fixture(&protected, &translations);
    let corpus = load_translation_corpus(&translations).unwrap();
    assert!(load_protected_layout_catalog(&protected, &corpus).is_ok());

    let segment_path = protected.join("segments/interface.json");
    let mut document: serde_json::Value =
        serde_json::from_slice(&fs::read(&segment_path).unwrap()).unwrap();
    document["entries"][0]["text"] = json!("바뀐 원문");
    fs::write(&segment_path, serde_json::to_vec_pretty(&document).unwrap()).unwrap();

    let error = load_protected_layout_catalog(&protected, &corpus)
        .err()
        .unwrap();
    assert!(error.to_string().contains("content hash changed"));
}

fn write_fixture(protected: &std::path::Path, translations: &std::path::Path) {
    fs::create_dir_all(protected.join("segments")).unwrap();
    fs::create_dir_all(translations.join("segments")).unwrap();
    let protected_segment = json!({
        "id": "interface",
        "surface": "interface_text",
        "entries": [{"id": "interface-01", "text": "原文", "byte_size": 4}]
    });
    let protected_segment_bytes = json_bytes(&protected_segment);
    let protected_segment_sha256 = sha256_hex(&protected_segment_bytes);
    fs::write(
        protected.join("segments/interface.json"),
        &protected_segment_bytes,
    )
    .unwrap();
    let protected_index = json!({
        "supported_source_sha256": "source",
        "segments": [{
            "id": "interface",
            "surface": "interface_text",
            "path": "segments/interface.json",
            "entry_count": 1,
            "content_sha256": protected_segment_sha256
        }]
    });
    let protected_index_bytes = json_bytes(&protected_index);
    fs::write(protected.join("index.json"), &protected_index_bytes).unwrap();

    let entries = vec![FixtureEntry {
        id: "interface-01".to_owned(),
        korean_text: vec!["한글".to_owned()],
        development_policy: "translate",
        status: "needs_human_review",
        notes: None,
        questions: Vec::new(),
    }];
    let draft_sha256 = sha256_hex(&serde_json::to_vec(&entries).unwrap());
    let draft_segment = json!({
        "id": "interface",
        "surface": "interface_text",
        "protected_segment_sha256": protected_segment_sha256,
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
        translations.join("segments/interface.json"),
        json_bytes(&draft_segment),
    )
    .unwrap();
    let draft_index = json!({
        "supported_source_sha256": "source",
        "protected_index_sha256": sha256_hex(&protected_index_bytes),
        "status": "needs_human_review",
        "translation_unit_count": 1,
        "translated_text_unit_count": 1,
        "preserved_source_control_unit_count": 0,
        "retained_source_audio_unit_count": 0,
        "producer_evidence_segments": [],
        "segments": [{
            "id": "interface",
            "surface": "interface_text",
            "path": "segments/interface.json",
            "entry_count": 1,
            "protected_segment_sha256": protected_segment_sha256,
            "draft_sha256": draft_sha256
        }]
    });
    fs::write(translations.join("index.json"), json_bytes(&draft_index)).unwrap();
}

fn json_bytes(value: &serde_json::Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}
