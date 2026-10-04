use super::*;

#[test]
fn sel1_layout_partitions_into_an_atlas_and_complete_screen_map() {
    assert_eq!(TILE_ATLAS_SIZE / TILE_SIZE, 57);
    assert_eq!(TILE_MAP_SIZE, 1_000);
    assert_eq!(TILE_ATLAS_SIZE + TILE_MAP_SIZE, 8_296);
}

#[test]
fn sel2_transfer_table_consumes_each_overlay_once() {
    for (index, transfer) in OVERLAY_TRANSFERS.iter().enumerate() {
        assert_eq!(transfer.0, index * OVERLAY_SIZE);
    }
    assert_eq!(OVERLAY_SIZE * OVERLAY_TRANSFERS.len(), 61_440);
}

#[test]
fn sel3_tables_preserve_the_one_unbound_range() {
    for (index, transfer) in SEL3_SPRITE_TRANSFERS.iter().enumerate() {
        let record_end = transfer.0 + transfer.3 * 5;
        let expected_end = SEL3_SPRITE_TRANSFERS
            .get(index + 1)
            .map(|next| next.0)
            .unwrap_or(SEL3_UNBOUND_START);
        assert_eq!(record_end, expected_end);
    }
    assert_eq!(SEL3_PANEL_TRANSFERS[0].0, SEL3_UNBOUND_END);
    for pair in SEL3_PANEL_TRANSFERS.windows(2) {
        assert_eq!(pair[0].0 + pair[0].2 * pair[0].3 * 4, pair[1].0);
    }
    let last = SEL3_PANEL_TRANSFERS.last().unwrap();
    assert_eq!(last.0 + last.2 * last.3 * 4, 0x9b40);
}
