use super::*;

#[test]
#[ignore = "requires the Galmuri and Mulmaru font files in assets/fonts or BAYOEN_WARS_FONT_DIR"]
fn canvas_centers_scaled_text_and_keeps_an_optional_border() {
    let canvas = TextCanvas::render(
        FontTarget::BakedDisplay16,
        &["가".to_owned()],
        64,
        48,
        2,
        true,
    )
    .unwrap();

    assert!(canvas.pixel(0, 0));
    assert!(canvas.pixel(63, 47));
    assert!(
        (8..40).any(|y| (16..48).any(|x| canvas.pixel(x, y))),
        "centered glyph must contain visible pixels"
    );
}

#[test]
fn compact_writer_encodes_foreground_and_background_colors() {
    let canvas = TextCanvas {
        width: 8,
        height: 1,
        pixels: vec![true, false, false, false, false, false, false, false],
    };
    let mut white_on_black = vec![0; 4];
    let mut black_on_white = vec![0; 4];

    write_compact_brgi(&mut white_on_black, 0, &canvas, 0, 15).unwrap();
    write_compact_brgi(&mut black_on_white, 0, &canvas, 15, 0).unwrap();

    assert_eq!(white_on_black, [0x80; 4]);
    assert_eq!(black_on_white, [0x7f; 4]);
}

#[test]
fn masked_writer_clears_the_destination_before_oring_colors() {
    let canvas = TextCanvas {
        width: 8,
        height: 1,
        pixels: vec![true, false, false, false, false, false, false, false],
    };
    let mut record = vec![0xff; 5];

    write_masked_brgi(&mut record, 0, &canvas, 0, 15).unwrap();

    assert_eq!(record, [0, 0x80, 0x80, 0x80, 0x80]);
}
