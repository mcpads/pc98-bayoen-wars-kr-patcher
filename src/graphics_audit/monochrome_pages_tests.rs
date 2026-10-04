use super::*;

#[test]
fn page_renderer_follows_script_order_and_line_breaks() {
    let mut glyphs = vec![0; GLYPH_SIZE * 2];
    glyphs[0] = 0x80;
    glyphs[GLYPH_SIZE] = 0x40;
    let page = MonochromeTextPage {
        id: "page".to_owned(),
        pointer_entry_file_offset: 0,
        file_offset: 0,
        byte_size: 0,
        content_sha256: String::new(),
        raw_hex: String::new(),
        glyph_count: 3,
        lines: vec![vec![1, 0], vec![0]],
    };

    let image = render_page(&glyphs, &page).unwrap();

    assert_eq!((image.width, image.height), (64, 64));
    assert_eq!(&image.pixels[3..2 * 3], &[255, 255, 255]);
    assert_eq!(&image.pixels[32 * 3..33 * 3], &[255, 255, 255]);
    assert_eq!(
        &image.pixels[(32 * 64) * 3..(32 * 64 + 1) * 3],
        &[255, 255, 255]
    );
}
