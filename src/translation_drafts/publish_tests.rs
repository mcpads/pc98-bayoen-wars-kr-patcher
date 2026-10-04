use std::fs;

use serde_json::json;

use super::*;
use crate::source_disk::sha256_hex;

#[test]
fn reviewed_proposals_publish_without_protected_source_content() {
    let directory = tempfile::tempdir().unwrap();
    let protected = directory.path().join("protected");
    let protected_segments = protected.join("segments");
    let proposals = directory.path().join("proposals");
    let output = directory.path().join("output");
    fs::create_dir_all(&protected_segments).unwrap();
    fs::create_dir(&proposals).unwrap();

    let text_bytes = json_bytes(&json!({
        "entries": [
            {"id": "text-01", "text": "原文", "raw_hex": "8c8b"},
            {"id": "control-01", "text": "\u{1b}[37m ", "raw_hex": "1b5b33376d20"}
        ]
    }));
    let voice_bytes = json_bytes(&json!({
        "entries": [{"id": "voice-01", "decoded_raw_hex": "00"}]
    }));
    let glyph_bytes = json_bytes(&json!({
        "entries": [{"id": "glyph-01", "raw_hex": "ff"}]
    }));
    fs::write(protected_segments.join("text.json"), &text_bytes).unwrap();
    fs::write(protected_segments.join("voice-samples.json"), &voice_bytes).unwrap();
    fs::write(protected_segments.join("glyphs.json"), &glyph_bytes).unwrap();
    fs::write(
        protected.join("index.json"),
        json_bytes(&json!({
            "supported_source_sha256": "source",
            "translation_segmentation_complete": true,
            "segments": [
                segment_reference("text", "dialogue", &text_bytes),
                segment_reference("voice-samples", "japanese_voice_performance", &voice_bytes),
                segment_reference("glyphs", "monochrome_glyph_atlas", &glyph_bytes)
            ]
        })),
    )
    .unwrap();
    fs::write(
        proposals.join("reviewed.json"),
        json_bytes(&json!({
            "batch_id": "reviewed",
            "entries": [
                {
                    "segment_id": "text",
                    "id": "text-01",
                    "korean_text": ["번역"],
                    "notes": null,
                    "questions": []
                },
                {
                    "segment_id": "text",
                    "id": "control-01",
                    "korean_text": [],
                    "notes": "색상 제어 문자열을 보존한다",
                    "questions": []
                },
                {
                    "segment_id": "voice-samples",
                    "id": "voice-01",
                    "korean_text": [],
                    "notes": "개발 빌드는 원음을 유지한다",
                    "questions": ["릴리즈 예외 승인이 필요하다"]
                }
            ],
            "review": {
                "method": "independent_second_llm",
                "outcome": "human_decision_required",
                "notes": ["보호 원문과 초벌을 독립 대조했다"]
            }
        })),
    )
    .unwrap();

    let report = publish_translation_drafts(&protected, &proposals, &output).unwrap();

    assert_eq!(report.translation_unit_count, 3);
    assert_eq!(report.translated_text_unit_count, 1);
    assert_eq!(report.preserved_source_control_unit_count, 1);
    assert_eq!(report.retained_source_audio_unit_count, 1);
    assert_eq!(report.unresolved_question_count, 1);
    let published = fs::read_to_string(output.join("segments/text.json")).unwrap();
    assert!(published.contains("번역"));
    assert!(!published.contains("原文"));
    assert!(!published.contains("raw_hex"));
}

fn json_bytes(value: &serde_json::Value) -> Vec<u8> {
    let mut bytes = serde_json::to_vec_pretty(value).unwrap();
    bytes.push(b'\n');
    bytes
}

fn segment_reference(id: &str, surface: &str, bytes: &[u8]) -> serde_json::Value {
    json!({
        "id": id,
        "surface": surface,
        "path": format!("segments/{id}.json"),
        "entry_count": if id == "text" { 2 } else { 1 },
        "content_sha256": sha256_hex(bytes)
    })
}
