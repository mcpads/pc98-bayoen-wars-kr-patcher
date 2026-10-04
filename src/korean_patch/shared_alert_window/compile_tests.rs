use super::*;

#[test]
fn shared_alert_output_geometry_covers_the_longest_compiled_line() {
    let layout = center_interface_window(7, 12, 0x2d18, 0x321a).unwrap();

    assert!(layout.output_inner_width_words >= layout.required_inner_width_words);
    assert_eq!(layout.output_inner_width_words, 13);
    assert_eq!(layout.output_frame_origin, 0x2d12);
    assert_eq!(layout.output_text_origin, 0x3214);
    assert_eq!(layout.output_frame_x_pixels, 0x0090);
}
