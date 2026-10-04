use super::*;

fn glyph(index: usize, file_offset: usize) -> GaijiGlyph {
    GaijiGlyph {
        index,
        character_code: 0x7621 + index as u16,
        shift_jis_code: 0xeb9f + index as u16,
        file_offset,
        byte_size: RECORD_SIZE,
        sha256: String::new(),
        meaning: crate::GaijiGlyphMeaning::Character { character: '字' },
    }
}

#[test]
fn sheet_uses_only_the_bitmap_body_of_each_ordered_record() {
    let mut program = vec![0_u8; RECORD_SIZE * 2];
    program[RECORD_PREFIX_SIZE] = 0x80;
    program[RECORD_SIZE + RECORD_PREFIX_SIZE + GLYPH_BITMAP_SIZE - 1] = 0x01;

    let image = render_gaiji_sheet(&program, &[glyph(0, 0), glyph(1, RECORD_SIZE)]).unwrap();

    assert_eq!(
        (image.width, image.height),
        (GLYPH_WIDTH * 2 + CELL_GAP, GLYPH_HEIGHT)
    );
    assert_eq!(&image.pixels[0..3], &[255, 255, 255]);
    let final_pixel = (image.width * image.height - 1) * 3;
    assert_eq!(
        &image.pixels[final_pixel..final_pixel + 3],
        &[255, 255, 255]
    );
}

#[test]
fn sheet_rejects_a_record_with_an_unknown_prefix() {
    let mut program = vec![0_u8; RECORD_SIZE];
    program[0] = 1;

    let error = render_gaiji_sheet(&program, &[glyph(0, 0)]).unwrap_err();

    assert!(error.to_string().contains("unexpected record prefix"));
}
