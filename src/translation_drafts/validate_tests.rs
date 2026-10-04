use super::*;
use crate::translation_drafts::model::ProposalEntry;

#[test]
fn source_free_check_rejects_japanese_in_notes_or_translation() {
    let mut entry = ProposalEntry {
        segment_id: "segment".to_owned(),
        id: "entry".to_owned(),
        korean_text: vec!["한국어".to_owned()],
        notes: None,
        questions: Vec::new(),
    };
    assert!(ensure_source_free(&entry).is_ok());

    entry.notes = Some("원문 テスト".to_owned());
    assert!(ensure_source_free(&entry).is_err());
}
