use super::*;

#[test]
fn short_header_preserves_high_then_low_nibble_order() {
    let sample = decode_sample(&[2, 0, 0xab, 0x10]).unwrap();

    assert_eq!(sample.header_size, 2);
    assert_eq!(sample.nibbles, [0x0a, 0x0b, 0x01, 0x00]);
}

#[test]
fn extended_header_requires_the_observed_marker() {
    assert!(decode_sample(&[1, 0, 0, 0, 0x5a, 0x34, 0x10]).is_ok());
    assert!(decode_sample(&[1, 0, 0, 0, 0, 0, 0x10]).is_err());
}

#[test]
fn unsupported_header_size_is_rejected() {
    assert!(decode_sample(&[1, 0, 0, 0x10]).is_err());
}
