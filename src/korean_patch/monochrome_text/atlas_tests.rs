use super::*;

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn atlas_assigns_one_blank_and_deterministic_first_use_slots() {
    let atlas = compile_atlas(&["가 나", "가!"], 5).unwrap();

    assert_eq!(atlas.glyph_indices[&' '], 0);
    assert_eq!(atlas.glyph_indices[&'가'], 1);
    assert_eq!(atlas.glyph_indices[&'나'], 2);
    assert_eq!(atlas.glyph_indices[&'!'], 3);
    assert_eq!(atlas.glyph_slots.len(), 4);
    assert_eq!(atlas.decoded.len(), 5 * GLYPH_BYTE_SIZE);
    assert_eq!(&atlas.decoded[..GLYPH_BYTE_SIZE], &[0; GLYPH_BYTE_SIZE]);
    assert_eq!(&atlas.decoded[4 * GLYPH_BYTE_SIZE..], &[0; GLYPH_BYTE_SIZE]);
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn narrative_atlas_uses_native_32px_raster_detail() {
    let atlas = compile_atlas(&["한"], 1).unwrap();
    let glyph = &atlas.decoded[..GLYPH_BYTE_SIZE];

    let has_mixed_two_by_two_block = (0..GLYPH_HEIGHT).step_by(2).any(|y| {
        (0..GLYPH_WIDTH).step_by(2).any(|x| {
            let pixels = [
                glyph_pixel(glyph, x, y),
                glyph_pixel(glyph, x + 1, y),
                glyph_pixel(glyph, x, y + 1),
                glyph_pixel(glyph, x + 1, y + 1),
            ];
            pixels.iter().any(|pixel| *pixel) && pixels.iter().any(|pixel| !*pixel)
        })
    });

    assert!(has_mixed_two_by_two_block);
}

fn glyph_pixel(glyph: &[u8], x: usize, y: usize) -> bool {
    glyph[y * (GLYPH_WIDTH / 8) + x / 8] & (0x80 >> (x % 8)) != 0
}

#[test]
fn atlas_rejects_capacity_overflow_and_non_space_whitespace() {
    assert!(
        compile_atlas(&["가나다"], 2)
            .unwrap_err()
            .to_string()
            .contains("needs 3 glyphs")
    );
    assert!(
        compile_atlas(&["가\t나"], 4)
            .unwrap_err()
            .to_string()
            .contains("unsupported whitespace")
    );
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn atlas_assigns_the_highest_non_control_index_without_crossing_the_runtime_buffer() {
    let text = (0..0xfe)
        .map(|index| char::from_u32(0xac00 + index).unwrap())
        .collect::<String>();
    let atlas = compile_atlas(&[&text], 0xfe).unwrap();

    assert_eq!(atlas.glyph_slots.len(), 0xfe);
    assert_eq!(atlas.glyph_slots.last().unwrap().index, 0xfd);
    assert_eq!(atlas.decoded.len(), 0x7f00);
}
