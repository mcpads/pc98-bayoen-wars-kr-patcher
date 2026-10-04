use super::*;

#[test]
fn brgi_tiles_require_complete_compact_records() {
    let error = render_compact_brgi_tiles(&[0; 127], 2, 16, 10).unwrap_err();

    assert!(error.to_string().contains("whole number"));
}

#[test]
fn compact_tiles_preserve_each_independent_plane_record() {
    let source = vec![0; 17 * 8 * 16 / 8 * 4];
    let image = render_compact_brgi_tiles(&source, 1, 16, 17).unwrap();

    assert_eq!(image.width, 640);
    assert_eq!(image.height, 400);
}
