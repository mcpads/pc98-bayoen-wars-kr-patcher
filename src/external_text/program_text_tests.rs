use super::ProgramTextBuilder;
use crate::external_text::catalog::ExternalTextStorage;

#[test]
fn duplicate_text_keeps_every_consumer_reference() {
    let bytes = b"message$";
    let mut builder = ProgramTextBuilder::new(
        "TEST.COM",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        bytes,
    );
    builder
        .add_com_dos_reference(0x100, 0x20, "first", None)
        .unwrap();
    builder
        .add_com_dos_reference(0x100, 0x30, "second", Some(2))
        .unwrap();

    let catalog = builder.finish().unwrap();
    assert_eq!(catalog.unique_text_count, 1);
    assert_eq!(catalog.reference_count, 2);
    assert_eq!(catalog.entries[0].id, "external-test-text-001");
    assert_eq!(catalog.entries[0].text, "message");
}

#[test]
fn unterminated_and_invalid_shift_jis_text_are_rejected() {
    let mut unterminated =
        ProgramTextBuilder::new("TEST.COM", ExternalTextStorage::OriginalFile, 4, b"text");
    assert!(
        unterminated
            .add_com_dos_reference(0x100, 0, "output", None)
            .is_err()
    );

    let mut invalid = ProgramTextBuilder::new(
        "TEST.SYS",
        ExternalTextStorage::OriginalFile,
        2,
        &[0x82, 0x00],
    );
    invalid.add_null_reference(0, 0, "output").unwrap();
    assert!(invalid.finish().is_err());
}

#[test]
fn explicit_new_id_does_not_renumber_established_entries() {
    let bytes = b"first$inserted$second$";
    let mut builder = ProgramTextBuilder::new(
        "TEST.COM",
        ExternalTextStorage::OriginalFile,
        bytes.len(),
        bytes,
    );
    builder
        .add_com_dos_reference(0x100, 0x10, "output", None)
        .unwrap();
    builder
        .add_com_dos_reference_with_id("external-test-text-003", 0x106, 0x20, "new_output", None)
        .unwrap();
    builder
        .add_com_dos_reference(0x10f, 0x30, "output", None)
        .unwrap();

    let catalog = builder.finish().unwrap();
    assert_eq!(
        catalog
            .entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>(),
        [
            "external-test-text-001",
            "external-test-text-003",
            "external-test-text-002",
        ]
    );
}
