use super::*;

fn mad_with_title_runtime_tables() -> Vec<u8> {
    let mut mad = vec![0; TITLE_PALETTE_TABLE_FILE_OFFSET + 65];
    let animation = encode_title_animation_frame_table(&SOURCE_TITLE_ANIMATION_FRAME_SOURCES);
    mad[TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET
        ..TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET + animation.len()]
        .copy_from_slice(&animation);
    let palette = encode_title_palette_table(&SOURCE_TITLE_PALETTE_RGB4).unwrap();
    mad[TITLE_PALETTE_TABLE_FILE_OFFSET..TITLE_PALETTE_TABLE_FILE_OFFSET + palette.len()]
        .copy_from_slice(&palette);
    mad
}

#[test]
fn runtime_tables_round_trip_the_source_palette_and_animation_sources() {
    let mad = mad_with_title_runtime_tables();

    assert_eq!(
        read_title_palette_rgb4(&mad).unwrap(),
        SOURCE_TITLE_PALETTE_RGB4
    );
    assert_eq!(
        read_title_animation_frame_sources(&mad).unwrap(),
        SOURCE_TITLE_ANIMATION_FRAME_SOURCES
    );
    assert!(!title_animation_is_disabled(&mad).unwrap());
}

#[test]
fn animation_is_disabled_only_when_every_consumed_frame_is_skipped() {
    let mut mad = mad_with_title_runtime_tables();
    let disabled = encode_title_animation_frame_table(&disabled_title_animation_frame_sources());
    mad[TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET
        ..TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET + disabled.len()]
        .copy_from_slice(&disabled);

    assert!(title_animation_is_disabled(&mad).unwrap());

    mad[TITLE_ANIMATION_FRAME_TABLE_FILE_OFFSET] = 0;
    assert!(!title_animation_is_disabled(&mad).unwrap());
}

#[test]
fn palette_reader_rejects_a_table_whose_index_order_changed() {
    let mut mad = mad_with_title_runtime_tables();
    mad[TITLE_PALETTE_TABLE_FILE_OFFSET + 4] = 2;

    let error = read_title_palette_rgb4(&mad).unwrap_err();

    assert!(format!("{error:#}").contains("index order changed"));
}
