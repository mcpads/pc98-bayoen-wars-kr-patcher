use super::*;
use crate::pc98_graphics::{encode_indexed_rgb4_palette, nearest_16_color_palette_index};

fn mad_with_character_palette() -> Vec<u8> {
    let encoded = encode_indexed_rgb4_palette(&SOURCE_CHARACTER_PALETTE_RGB4).unwrap();
    let mut mad = vec![0; CHARACTER_PALETTE_TABLE_FILE_OFFSET + encoded.len()];
    mad[CHARACTER_PALETTE_TABLE_FILE_OFFSET..CHARACTER_PALETTE_TABLE_FILE_OFFSET + encoded.len()]
        .copy_from_slice(&encoded);
    mad
}

#[test]
fn character_palette_is_read_from_the_runtime_table() {
    assert_eq!(
        read_character_palette_rgb4(&mad_with_character_palette()).unwrap(),
        SOURCE_CHARACTER_PALETTE_RGB4
    );
}

#[test]
fn normalized_green_maps_to_its_character_runtime_index() {
    let palette = expand_character_palette(&SOURCE_CHARACTER_PALETTE_RGB4);

    assert_eq!(palette[8], [119, 204, 0]);
    assert_eq!(nearest_16_color_palette_index([119, 204, 0], &palette), 8);
}

#[test]
fn character_palette_reader_rejects_changed_index_order() {
    let mut mad = mad_with_character_palette();
    mad[CHARACTER_PALETTE_TABLE_FILE_OFFSET + 4] = 2;

    let error = read_character_palette_rgb4(&mad).unwrap_err();

    assert!(format!("{error:#}").contains("index order changed"));
}
