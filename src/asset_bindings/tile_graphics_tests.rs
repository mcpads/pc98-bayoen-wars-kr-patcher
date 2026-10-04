use super::*;

#[test]
fn masked_mouse_records_cover_the_complete_asset() {
    assert_eq!(MOUSE_TILE_COUNT * MASKED_TILE_SIZE, MOUSE_CURSOR_OFFSET);
    assert_eq!(MOUSE_CURSOR_OFFSET + MOUSE_CURSOR_SIZE, 19_360);
}

#[test]
fn compact_tile_geometry_matches_the_observed_asset_sizes() {
    assert_eq!(FRAME_TILE_COUNT * COMPACT_TILE_SIZE, 23_296);
    assert_eq!(BACKGROUND_TILE_COUNT * COMPACT_TILE_SIZE, 32_768);
    assert_eq!(PORTRAIT_COUNT * PORTRAIT_SIZE, 36_864);
}

#[test]
fn tile_catalog_fails_closed_before_decoding_unverified_assets() {
    let error = catalog_tile_graphics(&vec![0; 0xd300], &BTreeMap::new()).unwrap_err();

    assert!(error.to_string().contains("mouse-graphics loader"));
}
