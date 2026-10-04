use super::*;

#[test]
fn mask_plane_is_skipped_and_color_planes_are_rendered() {
    let mut records = vec![0; 2 * 5 * 8];
    records[8..16].fill(0xff);
    records[48..56].fill(0xff);
    let image = render_masked_brgi_sheet(&records, 0, 2, 8, 8, 16, 0).unwrap();

    assert_eq!((image.width, image.height), (16, 8));
    assert_eq!(&image.pixels[..3], &image.pixels[8 * 3..8 * 3 + 3]);
}

#[test]
fn incomplete_masked_record_is_rejected() {
    let error = render_masked_brgi_sheet(&[0; 39], 0, 1, 8, 8, 8, 0).unwrap_err();

    assert!(error.to_string().contains("exceed"));
}
