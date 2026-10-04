use super::*;

#[test]
fn recovery_window_expands_save_and_restore_to_the_same_centered_geometry() {
    let layout = center_interface_window(11, 14, 0x3212, 0x3714).unwrap();
    let background_width = layout.output_inner_width_words + 2;

    assert_eq!(layout.output_inner_width_words, 15);
    assert_eq!(layout.output_frame_origin, 0x320e);
    assert_eq!(layout.output_text_origin, 0x3710);
    assert_eq!(layout.output_frame_x_pixels, 0x0070);
    assert_eq!(background_width, 17);
}
