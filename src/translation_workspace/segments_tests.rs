use super::*;

#[derive(Serialize)]
struct Entry {
    id: &'static str,
    source_text: &'static str,
}

#[test]
fn segment_reference_hashes_the_exact_protected_document() {
    let directory = tempfile::tempdir().unwrap();
    let entries = [Entry {
        id: "entry-001",
        source_text: "原文",
    }];

    let reference = write_segment(
        directory.path(),
        "fixture",
        "SOURCE.COM",
        "dialogue",
        &entries,
    )
    .unwrap();
    let bytes = fs::read(directory.path().join("fixture.json")).unwrap();

    assert_eq!(reference.entry_count, 1);
    assert_eq!(reference.content_sha256, sha256_hex(&bytes));
    assert!(String::from_utf8(bytes).unwrap().contains("原文"));
}
