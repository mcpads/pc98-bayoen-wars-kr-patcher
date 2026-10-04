use super::*;

fn image(width: usize, height: usize) -> RgbImage {
    RgbImage {
        width,
        height,
        pixels: (0..width * height)
            .flat_map(|pixel| [pixel as u8, 0, 0])
            .collect(),
    }
}

#[test]
fn crop_uses_the_exact_source_proven_rectangle() {
    let source = image(4, 3);

    let cropped = crop_image(&source, (1, 1, 2, 2)).unwrap();

    assert_eq!(cropped.width, 2);
    assert_eq!(cropped.height, 2);
    assert_eq!(cropped.pixels, [5, 0, 0, 6, 0, 0, 9, 0, 0, 10, 0, 0]);
}

#[test]
fn crop_outside_the_consumer_frame_is_rejected() {
    let error = crop_image(&image(4, 3), (3, 1, 2, 2)).unwrap_err();

    assert!(error.to_string().contains("exceeds"));
}

#[test]
fn visible_lines_omit_dos_control_bytes() {
    assert_eq!(lines("表示\u{7}\n次"), ["表示", "次"]);
    assert_eq!(lines("表示\r\n\u{7}"), ["表示"]);
}

#[test]
fn pair_heading_names_the_consumer_category() {
    assert_eq!(pair_heading("stage-selection"), "STAGE-SELECTION");
}
