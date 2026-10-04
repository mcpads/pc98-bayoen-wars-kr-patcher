use super::*;

fn image(width: usize, height: usize, color: [u8; 3]) -> RgbImage {
    RgbImage {
        width,
        height,
        pixels: color.repeat(width * height),
    }
}

#[test]
fn images_wrap_without_changing_their_pixels() {
    let sheet =
        compose_contact_sheet(vec![image(4, 2, [1, 2, 3]), image(4, 3, [4, 5, 6])], 6, 1).unwrap();

    assert_eq!((sheet.width, sheet.height), (4, 6));
    assert_eq!(&sheet.pixels[0..3], &[1, 2, 3]);
    assert_eq!(
        &sheet.pixels[3 * sheet.width * 3..3 * sheet.width * 3 + 3],
        &[4, 5, 6]
    );
}
