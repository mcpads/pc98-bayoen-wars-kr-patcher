use super::*;

#[test]
fn capture_window_centers_the_frame_text_and_background_together() {
    let layout = center_interface_window(9, 12, 0x3714, 0x3c16).unwrap();
    let background_width = layout.output_inner_width_words + 2;

    assert_eq!(layout.output_inner_width_words, 13);
    assert_eq!(layout.output_frame_origin, 0x3710);
    assert_eq!(layout.output_text_origin, 0x3c12);
    assert_eq!(layout.output_frame_x_pixels, 0x0080);
    assert_eq!(background_width, 15);
}
