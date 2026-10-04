use super::*;

#[test]
fn pointer_header_is_twelve_words() {
    assert_eq!(POINTER_TABLE_SIZE, 24);
}

#[test]
fn catalog_rejects_unverified_consumer_code() {
    let error = catalog_movement_tables(&vec![0; 0xae00], &BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains("movement-table loader"));
}
