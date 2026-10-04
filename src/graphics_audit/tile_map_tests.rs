use super::*;

#[test]
fn repeated_map_entries_reuse_the_selected_tile_pixels() {
    let mut tile = vec![0; 8 * 4];
    tile[8..16].fill(0xff);
    let image = render_compact_brgi_tile_map(&tile, 8, 8, &[0, 0], 2, 1).unwrap();

    assert_eq!((image.width, image.height), (16, 8));
    assert_eq!(&image.pixels[..3], &image.pixels[8 * 3..8 * 3 + 3]);
}

#[test]
fn missing_tile_reference_is_rejected() {
    let error = render_compact_brgi_tile_map(&[0; 32], 8, 8, &[1], 1, 1).unwrap_err();

    assert!(error.to_string().contains("missing tile"));
}
