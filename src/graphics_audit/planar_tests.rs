use super::*;

#[test]
fn strided_planes_form_brgi_palette_indices() {
    let source = [0x80, 0x40, 0x20, 0x10];

    let image = render_strided_brgi(&source, 8, 1, 1, 1).unwrap();

    assert_eq!(&image.pixels[0..3], &[0, 0, 170]);
    assert_eq!(&image.pixels[3..6], &[170, 0, 0]);
    assert_eq!(&image.pixels[6..9], &[0, 170, 0]);
    assert_eq!(&image.pixels[9..12], &[85, 85, 85]);
}

#[test]
fn compact_copy_rejects_a_destination_past_vram() {
    let mut screen = PlanarScreen::new();
    let error = screen
        .copy_compact_brgi(&[0; 8], 0, SCREEN_PLANE_BYTES - 1, 2, 1)
        .unwrap_err();

    assert!(error.to_string().contains("crosses graphics VRAM"));
}

#[test]
fn strided_planes_use_the_supplied_runtime_palette_indices() {
    let source = [0x80, 0x40, 0x20, 0x10];
    let mut palette = [[0; 3]; 16];
    palette[1] = [1, 2, 3];
    palette[2] = [4, 5, 6];
    palette[4] = [7, 8, 9];
    palette[8] = [10, 11, 12];

    let image = render_strided_brgi_with_palette(&source, 8, 1, 1, 1, &palette).unwrap();

    assert_eq!(
        &image.pixels[..12],
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
    );
}
