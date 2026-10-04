use super::*;

#[test]
fn song_sequence_contract_uses_the_complete_verified_file() {
    assert_eq!(SONG_SIZE, 14_800);
    assert_eq!(SONG_SIGNATURE_OFFSET, 0x0d);
    assert_eq!(SONG_SIGNATURE, b"MAIKO-HOSHINO ");
    assert_eq!(DRIVER_LOAD_CALL_OFFSET, 0x2846);
    assert_eq!(DRIVER_LOAD_SERVICE, 0x09);
    assert_eq!(DRIVER_LOAD_INTERRUPT, 0x7f);
}

#[test]
fn catalog_rejects_missing_sound_data() {
    let error = catalog_sound_data(&BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains("SONG.DAT"));
}

#[test]
fn byte_search_reports_every_overlapping_match() {
    assert_eq!(find_all(b"aaaa", b"aa"), vec![0, 1, 2]);
}
