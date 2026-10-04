use super::*;

#[test]
fn bitmap_is_wrapped_in_the_target_34_byte_record() {
    let mut bitmap = [0_u8; GLYPH_BYTES];
    bitmap[0] = 0x80;

    let bytes = GaijiRecord::from_bitmap(bitmap).to_bytes();

    assert_eq!(&bytes[..2], &[0, 0]);
    assert_eq!(&bytes[2..], &bitmap);
}

#[test]
fn non_target_prefix_is_rejected() {
    let mut bytes = [0_u8; GAIJI_RECORD_SIZE];
    bytes[0] = 2;

    let error = GaijiRecord::parse(&bytes)
        .unwrap()
        .require_target_format()
        .unwrap_err();

    assert!(error.to_string().contains("00 00 format"));
}

#[test]
fn wrong_record_size_is_rejected() {
    let error = GaijiRecord::parse(&[0_u8; GAIJI_RECORD_SIZE - 1]).unwrap_err();

    assert!(error.to_string().contains("must be 34 bytes"));
}
