use super::TRANSFERS;

#[test]
fn defeat_transfers_consume_the_decoded_asset_without_gaps() {
    for pair in TRANSFERS.windows(2) {
        let (source, _, width_words, height) = pair[0];
        assert_eq!(source + width_words * 2 * height * 4, pair[1].0);
    }
    assert_eq!(TRANSFERS.last().unwrap().0, 0x1d00);
}
