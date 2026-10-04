use super::*;

#[test]
fn tile_location_index_keeps_every_map_reference() {
    let mut map = vec![0; TILE_COLUMNS * TILE_ROWS];
    map[1] = 2;
    map[TILE_COLUMNS + 3] = 2;

    let locations = tile_locations(&map).unwrap();

    assert_eq!(locations[&2], [(0, 1), (1, 3)]);
    assert_eq!(locations[&0].len(), TILE_COLUMNS * TILE_ROWS - 2);
}

#[test]
fn tile_location_index_rejects_an_out_of_atlas_reference() {
    let mut map = vec![0; TILE_COLUMNS * TILE_ROWS];
    map[0] = TILE_COUNT as u8;

    assert!(
        tile_locations(&map)
            .unwrap_err()
            .to_string()
            .contains("references tile")
    );
}
