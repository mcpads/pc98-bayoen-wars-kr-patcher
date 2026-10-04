use super::*;

#[test]
fn overlapping_transfer_ranges_count_each_source_byte_once() {
    let merged = merge_ranges([0..10, 4..12, 20..24, 24..30]);

    assert_eq!(merged, vec![0..12, 20..30]);
    assert_eq!(merged.iter().map(Range::len).sum::<usize>(), 22);
}

#[test]
fn complement_preserves_internal_and_trailing_unbound_bytes() {
    assert_eq!(
        complement_ranges(32, &[0..12, 20..30]),
        vec![12..20, 30..32]
    );
}

#[test]
fn consumer_signatures_fail_closed_before_table_parsing() {
    let error = catalog_character_sprites(&vec![0; 0x9400], &BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains("dynamic character-asset loader"));
}
