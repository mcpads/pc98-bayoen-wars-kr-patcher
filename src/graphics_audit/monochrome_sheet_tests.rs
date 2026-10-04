use super::*;

#[test]
fn bit_order_and_tile_placement_match_the_blitter() {
    let image = render_monochrome_sheet(&[0x80, 0x01], 8, 1, 2).unwrap();

    assert_eq!((image.width, image.height), (16, 1));
    assert_eq!(&image.pixels[..3], &[255, 255, 255]);
    assert_eq!(&image.pixels[15 * 3..16 * 3], &[255, 255, 255]);
}

#[test]
fn partial_tile_is_rejected() {
    let error = render_monochrome_sheet(&[0; 3], 16, 2, 1).unwrap_err();

    assert!(error.to_string().contains("inside a tile"));
}

#[test]
fn contact_sheet_keeps_a_visible_gap_between_indexed_tiles() {
    let source = [0x80, 0x01];

    let image = render_monochrome_contact_sheet(&source, 8, 1, 2, 2).unwrap();

    assert_eq!((image.width, image.height), (18, 1));
    assert_eq!(&image.pixels[0..3], &[255, 255, 255]);
    assert_eq!(&image.pixels[8 * 3..10 * 3], &[32, 32, 32, 32, 32, 32]);
    assert_eq!(&image.pixels[17 * 3..18 * 3], &[255, 255, 255]);
}
