use super::*;

#[test]
fn producer_bytes_do_not_count_as_a_consumer_reference() {
    let installer = BTreeMap::from([
        (GAIJI_PRODUCER.to_owned(), vec![0xec, 0x4a]),
        ("MAD.COM".to_owned(), vec![0x82, 0xf1]),
    ]);

    let audit =
        require_unreferenced_gaiji_code(0xec4a, &installer, &BTreeMap::new(), &BTreeMap::new())
            .unwrap();

    assert_eq!(audit.audited_file_count, 1);
    assert!(audit.occurrences.is_empty());
}

#[test]
fn any_exact_code_pair_in_a_consumer_blocks_the_slot() {
    let installer = BTreeMap::from([
        (GAIJI_PRODUCER.to_owned(), Vec::new()),
        ("MAD.COM".to_owned(), vec![0, 0xec, 0x4a, 0]),
    ]);

    let error =
        require_unreferenced_gaiji_code(0xec4a, &installer, &BTreeMap::new(), &BTreeMap::new())
            .unwrap_err();

    assert!(error.to_string().contains("MAD.COM at offset 0x1"));
}
