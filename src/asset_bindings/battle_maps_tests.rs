use super::*;

#[test]
fn battle_map_catalog_rejects_missing_consumer_code() {
    let error = catalog_battle_maps(&vec![0; 0xab00], &BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains("battle-map loader"));
}

#[test]
fn overview_is_four_complete_planar_images() {
    assert_eq!(OVERVIEW_BYTE_SIZE, 3_200);
    assert_eq!(OVERVIEW_BYTE_SIZE, OVERVIEW_PLANE_SIZE * 4);
    assert_eq!(DECODED_MAP_SIZE - OVERVIEW_BYTE_SIZE, 1_302);
}
