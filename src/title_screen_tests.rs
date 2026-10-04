use super::*;
use crate::title_runtime::{SOURCE_TITLE_PALETTE_RGB4, expand_title_palette};

fn source_title_palette() -> [[u8; 3]; DIGITAL_RGBI_COLOR_COUNT] {
    expand_title_palette(&SOURCE_TITLE_PALETTE_RGB4)
}

#[test]
fn replacing_title_content_changes_only_consumer_owned_transfer_bytes() {
    let mut decoded = vec![0x5a; TITLE_DECODED_SIZE];
    let original = decoded.clone();
    let indices = vec![0x0f; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];

    replace_title_content(&mut decoded, &indices).unwrap();

    let mut owned = vec![false; TITLE_DECODED_SIZE];
    for transfer in TITLE_CONTENT_TRANSFERS {
        let size = transfer.width_bytes * transfer.height * 4;
        owned[transfer.source_offset..transfer.source_offset + size].fill(true);
    }
    for (offset, (before, after)) in original.iter().zip(&decoded).enumerate() {
        if owned[offset] {
            assert_eq!(*after, 0xff, "owned byte {offset:#x} was not replaced");
        } else {
            assert_eq!(before, after, "unowned byte {offset:#x} changed");
        }
    }
}

#[test]
fn rendering_reads_the_same_screen_coordinates_that_replacement_writes() {
    let mut decoded = vec![0; TITLE_DECODED_SIZE];
    let mut indices = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    let point = (
        TITLE_CONTENT_TRANSFERS[0].x() + 3,
        TITLE_CONTENT_TRANSFERS[0].y() + 5,
    );
    indices[point.1 * TITLE_SCREEN_WIDTH + point.0] = 0x0e;

    replace_title_content(&mut decoded, &indices).unwrap();
    let screen = render_title_screen(&decoded).unwrap();

    let offset = (point.1 * TITLE_SCREEN_WIDTH + point.0) * 3;
    assert_eq!(
        &screen.pixels[offset..offset + 3],
        &digital_rgbi_palette()[0x0e]
    );
}

#[test]
fn artwork_replacement_preserves_source_background_and_copyright_bytes() {
    let mut decoded = vec![0x5a; TITLE_DECODED_SIZE];
    let original = decoded.clone();
    let indices = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];

    replace_title_artwork_preserving_source_background(&mut decoded, &indices, &[0, 1]).unwrap();

    let background_size = BACKGROUND_WIDTH_BYTES * BACKGROUND_HEIGHT * 4;
    assert_eq!(
        &decoded[BACKGROUND_SOURCE_OFFSET..BACKGROUND_SOURCE_OFFSET + background_size],
        &original[BACKGROUND_SOURCE_OFFSET..BACKGROUND_SOURCE_OFFSET + background_size]
    );
    assert_eq!(
        &decoded[TITLE_COPYRIGHT_TRANSFER.source_offset..BACKGROUND_SOURCE_OFFSET],
        &original[TITLE_COPYRIGHT_TRANSFER.source_offset..BACKGROUND_SOURCE_OFFSET]
    );
}

#[test]
fn palette_render_uses_the_supplied_title_colors() {
    let mut decoded = vec![0; TITLE_DECODED_SIZE];
    replace_title_artwork_preserving_source_background(
        &mut decoded,
        &vec![1; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT],
        &[1],
    )
    .unwrap();

    let screen = render_title_screen_with_palette(&decoded, &source_title_palette()).unwrap();
    let x = TITLE_CONTENT_TRANSFERS[0].x();
    let y = TITLE_CONTENT_TRANSFERS[0].y();
    let offset = (y * TITLE_SCREEN_WIDTH + x) * 3;

    assert_eq!(&screen.pixels[offset..offset + 3], &[0x66, 0x00, 0x00]);
}

#[test]
fn animation_frame_overwrites_the_consumer_rectangle_before_palette_rendering() {
    let title = vec![0; TITLE_DECODED_SIZE];
    let mut animation = vec![0; ANIMATION_WIDTH_BYTES * ANIMATION_HEIGHT * 4];
    animation[0] = 0x80;
    let mut palette = [[0; 3]; DIGITAL_RGBI_COLOR_COUNT];
    palette[1] = [0x66, 0, 0];

    let indices = render_title_color_indices_with_animation(&title, &animation, 0).unwrap();
    let screen = render_title_screen_with_animation(&title, &animation, 0, &palette).unwrap();
    let x = ANIMATION_DESTINATION_OFFSET % SCREEN_ROW_BYTES * 8;
    let y = ANIMATION_DESTINATION_OFFSET / SCREEN_ROW_BYTES;
    let offset = (y * TITLE_SCREEN_WIDTH + x) * 3;

    assert_eq!(indices[y * TITLE_SCREEN_WIDTH + x], 1);
    assert_eq!(indices[y * TITLE_SCREEN_WIDTH + x + 1], 0);
    assert_eq!(&screen.pixels[offset..offset + 3], &[0x66, 0, 0]);
    assert_eq!(&screen.pixels[offset + 3..offset + 6], &[0, 0, 0]);
}

#[test]
fn replacement_rejects_anything_other_than_one_full_screen_of_indices() {
    let mut decoded = vec![0; TITLE_DECODED_SIZE];

    let error = replace_title_content(&mut decoded, &[0; 1]).unwrap_err();

    assert!(error.to_string().contains("one color index per 640x400"));
}

#[test]
fn artwork_replacement_rejects_invalid_screen_before_mutating_the_title() {
    let mut decoded = vec![0x5a; TITLE_DECODED_SIZE];
    let original = decoded.clone();

    replace_title_artwork_preserving_source_background(&mut decoded, &[0; 1], &[0, 1]).unwrap_err();

    assert_eq!(decoded, original);
}

#[test]
fn artwork_replacement_rejects_invalid_palette_index_before_mutating_the_title() {
    let mut decoded = vec![0x5a; TITLE_DECODED_SIZE];
    let original = decoded.clone();
    let indices = vec![16; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];

    replace_title_artwork_preserving_source_background(&mut decoded, &indices, &[0, 1])
        .unwrap_err();

    assert_eq!(decoded, original);
}

#[test]
fn artwork_replacement_rejects_visible_pixels_beyond_the_right_consumer_edge() {
    let mut decoded = vec![0x5a; TITLE_DECODED_SIZE];
    let original = decoded.clone();
    let mut indices = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    indices[100 * TITLE_SCREEN_WIDTH + 576] = 2;

    let error = replace_title_artwork_preserving_source_background(&mut decoded, &indices, &[0, 1])
        .unwrap_err();

    assert!(error.to_string().contains("outside its consumer transfers"));
    assert_eq!(decoded, original);
}

#[test]
fn artwork_replacement_rejects_visible_pixels_below_the_consumer_transfers() {
    let mut decoded = vec![0x5a; TITLE_DECODED_SIZE];
    let original = decoded.clone();
    let mut indices = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    indices[320 * TITLE_SCREEN_WIDTH + 300] = 2;

    let error = replace_title_artwork_preserving_source_background(&mut decoded, &indices, &[0, 1])
        .unwrap_err();

    assert!(error.to_string().contains("outside its consumer transfers"));
    assert_eq!(decoded, original);
}

#[test]
fn artwork_replacement_accepts_visible_pixels_at_the_right_consumer_edge() {
    let mut decoded = vec![0; TITLE_DECODED_SIZE];
    let mut indices = vec![0; TITLE_SCREEN_WIDTH * TITLE_SCREEN_HEIGHT];
    indices[100 * TITLE_SCREEN_WIDTH + 575] = 2;

    replace_title_artwork_preserving_source_background(&mut decoded, &indices, &[0, 1]).unwrap();

    let palette = source_title_palette();
    let screen = render_title_screen_with_palette(&decoded, &palette).unwrap();
    let offset = (100 * TITLE_SCREEN_WIDTH + 575) * 3;
    assert_eq!(&screen.pixels[offset..offset + 3], &palette[2]);
}
