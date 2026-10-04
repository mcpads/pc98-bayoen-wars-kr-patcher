use super::*;

#[test]
fn sel1_panel_binds_map_rows_and_only_the_referenced_tiles() {
    let mut decoded = vec![0; SEL1_TILE_ATLAS_SIZE + SEL1_TILE_COLUMNS * SEL1_TILE_ROWS];
    decoded[SEL1_TILE_ATLAS_SIZE] = 2;
    decoded[SEL1_TILE_ATLAS_SIZE + SEL1_TILE_COLUMNS] = 3;

    let fragments = catalog_sel1_panel_fragments(&decoded, 0).unwrap();

    assert_eq!(fragments[0].role, "tile_map_row_00");
    assert_eq!(fragments[1].role, "tile_map_row_01");
    assert_eq!(
        fragments
            .iter()
            .skip(2)
            .map(|fragment| fragment.role.as_str())
            .collect::<Vec<_>>(),
        ["tile_00", "tile_02", "tile_03"]
    );
}

#[test]
fn sel3_name_records_are_exact_and_contiguous() {
    for (pair, next) in SEL3_NAME_TRANSFERS
        .windows(2)
        .zip(SEL3_NAME_TRANSFERS.iter().skip(1))
    {
        let (offset, _, _, plane_stride) = pair[0];
        assert_eq!(offset + plane_stride * 5, next.0);
    }
    let (offset, _, _, plane_stride) = SEL3_NAME_TRANSFERS[9];
    assert_eq!(offset + plane_stride * 5, 0x4c40);
}

#[test]
fn screen_coordinates_follow_pc98_plane_offsets() {
    assert_eq!(
        screen_region("base", 0x2304, 112, 32),
        BakedTextScreenRegion {
            variant: "base".to_owned(),
            x: 32,
            y: 112,
            width: 112,
            height: 32,
        }
    );
}
