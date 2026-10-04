use super::*;

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn fixed_cell_renderer_keeps_line_geometry_and_blank_cells() {
    let image = render_fixed_cell_lines(&["한 A".to_owned(), "글".to_owned()]).unwrap();

    assert_eq!(image.width, 3 * CELL_SIZE);
    assert_eq!(image.height, 2 * CELL_SIZE);
    assert!(image.pixels.iter().any(|pixel| *pixel != 0));
    for y in 0..CELL_SIZE {
        let blank_cell = (y * image.width + CELL_SIZE) * 3;
        assert!(
            image.pixels[blank_cell..blank_cell + CELL_SIZE * 3]
                .iter()
                .all(|pixel| *pixel == 0)
        );
    }
}

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn fixed_cell_renderer_applies_consumer_owned_glyph_overrides() {
    let overrides = [('…', [0xff_u8; GLYPH_BYTES])].into_iter().collect();

    let image = render_fixed_cell_lines_with_overrides(&["…".to_owned()], &overrides).unwrap();

    assert!(image.pixels.iter().all(|channel| *channel == 255));
}

#[test]
fn bitmap_renderer_uses_most_significant_bit_first() {
    let mut image = RgbImage {
        width: CELL_SIZE,
        height: CELL_SIZE,
        pixels: vec![0; CELL_SIZE * CELL_SIZE * 3],
    };
    let mut bitmap = [0; GLYPH_BYTES];
    bitmap[0] = 0x80;

    draw_bitmap(&mut image, 0, 0, &bitmap);

    assert_eq!(&image.pixels[..3], &[255, 255, 255]);
    assert_eq!(&image.pixels[3..6], &[0, 0, 0]);
}
