use super::place_records_in_source_storage;
use crate::external_text::{
    ExternalProgramTextCatalog, ExternalTextEntry, ExternalTextStorage, ExternalTextTerminator,
};
use crate::korean_patch::playback_driver_text::model::CompiledPlaybackDriverTextEntry;

fn source_entry(id: &str, offset: usize, byte_size: usize) -> ExternalTextEntry {
    ExternalTextEntry {
        id: id.to_owned(),
        text_offset: offset,
        runtime_address: offset + 0x100,
        terminator: ExternalTextTerminator::DosDollar,
        byte_size,
        raw_sha256: String::new(),
        raw_hex: String::new(),
        text: String::new(),
        references: Vec::new(),
    }
}

fn record(id: &str, order: usize, bytes: &[u8]) -> CompiledPlaybackDriverTextEntry {
    CompiledPlaybackDriverTextEntry {
        id: id.to_owned(),
        source_order: order,
        original_file_offset: 0,
        file_offset: 0,
        com_address: 0,
        lines: Vec::new(),
        bytes: bytes.to_vec(),
    }
}

#[test]
fn best_fit_reuses_the_complete_owned_slot_population() {
    let unpacked = b"abc$!12345678$";
    let source = ExternalProgramTextCatalog {
        file_name: "TEST.COM".to_owned(),
        storage: ExternalTextStorage::UnpackedComImage,
        packed_size: 0,
        text_storage_size: unpacked.len(),
        unique_text_count: 2,
        reference_count: 0,
        entries: vec![source_entry("small", 0, 3), source_entry("large", 5, 8)],
    };
    let mut entries = vec![
        record("large-record", 0, b"ABCDEFG$"),
        record("small-record", 1, b"xy$"),
    ];

    let (writes, capacity, used) =
        place_records_in_source_storage(unpacked, &source, &mut entries).unwrap();

    assert_eq!(capacity, 13);
    assert_eq!(used, 11);
    assert_eq!(entries[0].file_offset, 5);
    assert_eq!(entries[1].file_offset, 0);
    assert_eq!(writes[0].replacement, b"xy$$");
    assert_eq!(writes[1].replacement, b"ABCDEFG$$");
}

#[test]
fn adjacent_owned_slots_form_one_contiguous_storage_span() {
    let unpacked = b"abc$def$";
    let source = ExternalProgramTextCatalog {
        file_name: "TEST.COM".to_owned(),
        storage: ExternalTextStorage::UnpackedComImage,
        packed_size: 0,
        text_storage_size: unpacked.len(),
        unique_text_count: 2,
        reference_count: 0,
        entries: vec![source_entry("one", 0, 3), source_entry("two", 4, 3)],
    };
    let mut entries = vec![record("combined", 0, b"1234567$")];

    let (writes, _, _) = place_records_in_source_storage(unpacked, &source, &mut entries).unwrap();

    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].expected_original, unpacked);
    assert_eq!(writes[0].replacement, b"1234567$");
}

#[test]
fn a_record_larger_than_every_owned_slot_is_rejected() {
    let unpacked = b"abc$!def$";
    let source = ExternalProgramTextCatalog {
        file_name: "TEST.COM".to_owned(),
        storage: ExternalTextStorage::UnpackedComImage,
        packed_size: 0,
        text_storage_size: unpacked.len(),
        unique_text_count: 2,
        reference_count: 0,
        entries: vec![source_entry("one", 0, 3), source_entry("two", 5, 3)],
    };
    let mut entries = vec![record("too-large", 0, b"12345$")];

    assert!(place_records_in_source_storage(unpacked, &source, &mut entries).is_err());
}
