use super::*;

#[test]
fn odd_required_growth_adds_one_spare_cell_and_stays_centered() {
    let layout = center_interface_window(9, 12, 0x3714, 0x3c16).unwrap();

    assert_eq!(layout.output_inner_width_words, 13);
    assert_eq!(layout.horizontal_shift_words, 2);
    assert_eq!(layout.output_frame_origin, 0x3710);
    assert_eq!(layout.output_text_origin, 0x3c12);
    assert_eq!(layout.output_frame_x_pixels, 0x0080);
}

#[test]
fn six_cell_growth_moves_both_origins_three_cells_left() {
    let layout = center_interface_window(7, 12, 0x2d18, 0x321a).unwrap();

    assert_eq!(layout.output_inner_width_words, 13);
    assert_eq!(layout.horizontal_shift_words, 3);
    assert_eq!(layout.output_frame_origin, 0x2d12);
    assert_eq!(layout.output_text_origin, 0x3214);
    assert_eq!(layout.output_frame_x_pixels, 0x0090);
}

#[test]
fn fitting_text_preserves_the_source_geometry() {
    let layout = center_interface_window(11, 10, 0x3212, 0x3714).unwrap();

    assert_eq!(layout.output_inner_width_words, 11);
    assert_eq!(layout.horizontal_shift_words, 0);
    assert_eq!(layout.output_frame_origin, 0x3212);
    assert_eq!(layout.output_text_origin, 0x3714);
}

#[test]
fn expansion_that_crosses_a_graphics_row_is_rejected() {
    assert!(center_interface_window(7, 16, 0x2d08, 0x320a).is_err());
}
