use super::*;

#[test]
fn masked_record_populations_reconcile_with_observed_sizes() {
    assert_eq!(24 * MASKED_SPRITE_SIZE + 128, 34_688);
    assert_eq!(24 * MASKED_SPRITE_SIZE, 34_560);
    assert_eq!(5 * MASKED_SPRITE_SIZE, 7_200);
}

#[test]
fn malformed_window_table_is_rejected() {
    let error =
        parse_window_transfers(&vec![0; WINDOW_TRANSFER_TABLE_FILE_OFFSET + 8]).unwrap_err();

    assert!(error.to_string().contains("empty battle-window transfer"));
}
